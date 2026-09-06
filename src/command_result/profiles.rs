use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProfileItem {
    pub name: String,
    pub server: String,
    pub active: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ProfileListSuccess {
    pub active_profile: Option<String>,
    pub profiles: Vec<ProfileItem>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ProfileRemoveSuccess {
    pub name: String,
    pub server: String,
    pub removed: bool,
    pub active_profile: Option<String>,
}
