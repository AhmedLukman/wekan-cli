use crate::client::{
    WekanClient,
    exports::{ExportError, ExportMetadata, ExportOptions},
};
use std::fs::File;
impl WekanClient {
    pub async fn export_board(
        &self,
        board: &str,
        options: ExportOptions<'_>,
        file: &mut File,
    ) -> Result<ExportMetadata, ExportError> {
        let mut segments = vec![board];
        segments.extend_from_slice(options.format.suffix());
        self.export_resource(&segments, options, file, false).await
    }
}
