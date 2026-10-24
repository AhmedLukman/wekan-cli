use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
};

use directories::ProjectDirs;
use fs4::FileExt;
use serde::{Deserialize, Serialize};

use super::{Release, file_error};
use crate::error::AppError;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CacheEntry {
    pub last_attempt: Option<u64>,
    pub last_notified: Option<u64>,
    pub release: Option<Release>,
}

pub(crate) trait UpdateCache {
    fn load(&self) -> Result<CacheEntry, AppError>;
    fn save(&mut self, entry: &CacheEntry) -> Result<(), AppError>;
}

/// Hold the cache lease through the optional prompt so concurrent starts do
/// not issue duplicate requests or consume each other's terminal input.
pub(crate) struct FileCache {
    path: PathBuf,
    _lease: File,
}

impl FileCache {
    pub fn production() -> Result<Self, AppError> {
        let dirs = ProjectDirs::from("org", "Wekan", "wekan-cli")
            .ok_or_else(|| AppError::configuration("no platform cache directory is available"))?;
        Self::open(dirs.cache_dir().join("updates"))
    }

    pub(super) fn open(directory: PathBuf) -> Result<Self, AppError> {
        fs::create_dir_all(&directory)
            .map_err(|e| file_error("create update cache directory", e))?;
        let lease = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("check.lock"))
            .map_err(|e| file_error("open update cache lock", e))?;
        FileExt::try_lock(&lease).map_err(|e| file_error("lock update cache", e))?;
        Ok(Self {
            path: directory.join("check.json"),
            _lease: lease,
        })
    }
}

impl UpdateCache for FileCache {
    fn load(&self) -> Result<CacheEntry, AppError> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(CacheEntry::default());
            }
            Err(error) => return Err(file_error("read update cache", error)),
        };
        let mut bytes = Vec::new();
        file.take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| file_error("read update cache", e))?;
        // A corrupt or obsolete cache is replaceable; never block startup on it.
        Ok(if bytes.len() > 1024 * 1024 {
            CacheEntry::default()
        } else {
            serde_json::from_slice(&bytes).unwrap_or_default()
        })
    }

    fn save(&mut self, entry: &CacheEntry) -> Result<(), AppError> {
        let mut staging =
            tempfile::NamedTempFile::new_in(self.path.parent().expect("cache has a parent"))
                .map_err(|e| file_error("stage update cache", e))?;
        let bytes = serde_json::to_vec(entry).expect("cache is serializable");
        staging
            .write_all(&bytes)
            .and_then(|()| staging.as_file().sync_all())
            .map_err(|e| file_error("write update cache", e))?;
        staging
            .persist(&self.path)
            .map_err(|e| file_error("save update cache", e))?;
        Ok(())
    }
}
