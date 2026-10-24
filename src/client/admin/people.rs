//! Site-admin domain reporting and organization/team feature operations, v12.57.
use super::Object;
use crate::client::{
    ClientError, WekanClient, datetime::validate_response_date, transport::decode_success,
};
use reqwest::{Method, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureField {
    SharedTemplates,
    PropagateMembersToBoards,
    SyncMembersFromAuth,
}

impl FeatureField {
    fn org_field(self) -> &'static str {
        match self {
            Self::SharedTemplates => "orgSharedTemplates",
            Self::PropagateMembersToBoards => "orgPropagateMembersToBoards",
            Self::SyncMembersFromAuth => "orgSyncMembersFromAuth",
        }
    }
    fn team_field(self) -> &'static str {
        match self {
            Self::SharedTemplates => "teamSharedTemplates",
            Self::PropagateMembersToBoards => "teamPropagateMembersToBoards",
            Self::SyncMembersFromAuth => "teamSyncMembersFromAuth",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DomainCount {
    pub domain: String,
    pub count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationSummary {
    #[serde(rename(deserialize = "_id"))]
    pub organization_id: String,
    #[serde(rename(deserialize = "orgDisplayName"))]
    pub display_name: Option<String>,
    #[serde(rename(deserialize = "orgShortName"))]
    pub short_name: Option<String>,
    #[serde(rename(deserialize = "orgSharedTemplates"))]
    pub shared_templates: bool,
    #[serde(rename(deserialize = "orgPropagateMembersToBoards"))]
    pub propagate_members_to_boards: bool,
    #[serde(rename(deserialize = "orgSyncMembersFromAuth"))]
    pub sync_members_from_auth: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeamSummary {
    #[serde(rename(deserialize = "_id"))]
    pub team_id: String,
    #[serde(rename(deserialize = "teamDisplayName"))]
    pub display_name: Option<String>,
    #[serde(rename(deserialize = "teamShortName"))]
    pub short_name: Option<String>,
    #[serde(rename(deserialize = "teamSharedTemplates"))]
    pub shared_templates: bool,
    #[serde(rename(deserialize = "teamPropagateMembersToBoards"))]
    pub propagate_members_to_boards: bool,
    #[serde(rename(deserialize = "teamSyncMembersFromAuth"))]
    pub sync_members_from_auth: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationDocument {
    #[serde(rename(deserialize = "_id"))]
    pub organization_id: String,
    #[serde(rename(deserialize = "orgDisplayName"))]
    pub display_name: Option<String>,
    #[serde(rename(deserialize = "orgDesc"))]
    pub desc: Option<String>,
    #[serde(rename(deserialize = "orgShortName"))]
    pub short_name: Option<String>,
    #[serde(rename(deserialize = "orgAutoAddUsersWithDomainName"))]
    pub auto_add_users_with_domain_name: Option<String>,
    #[serde(rename(deserialize = "orgWebsite"))]
    pub website: Option<String>,
    #[serde(rename(deserialize = "orgIsActive"))]
    pub is_active: Option<bool>,
    #[serde(rename(deserialize = "orgSharedTemplates"))]
    pub shared_templates: Option<bool>,
    #[serde(rename(deserialize = "orgPropagateMembersToBoards"))]
    pub propagate_members_to_boards: Option<bool>,
    #[serde(rename(deserialize = "orgSyncMembersFromAuth"))]
    pub sync_members_from_auth: Option<bool>,
    #[serde(rename(deserialize = "orgDomains"))]
    pub domains: Option<String>,
    #[serde(rename(deserialize = "orgProductName"))]
    pub product_name: Option<String>,
    #[serde(rename(deserialize = "orgThemeColor"))]
    pub theme_color: Option<String>,
    #[serde(rename(deserialize = "orgThemeCustomColors"))]
    pub theme_custom_colors: Option<Vec<Option<String>>>,
    #[serde(rename(deserialize = "orgCustomLoginLogoImageUrl"))]
    pub custom_login_logo_image_url: Option<String>,
    #[serde(rename(deserialize = "orgCustomLoginLogoLinkUrl"))]
    pub custom_login_logo_link_url: Option<String>,
    #[serde(rename(deserialize = "orgTextBelowCustomLoginLogo"))]
    pub text_below_custom_login_logo: Option<String>,
    #[serde(rename(deserialize = "orgCustomTopLeftCornerLogoImageUrl"))]
    pub custom_top_left_corner_logo_image_url: Option<String>,
    #[serde(rename(deserialize = "orgCustomTopLeftCornerLogoLinkUrl"))]
    pub custom_top_left_corner_logo_link_url: Option<String>,
    #[serde(rename(deserialize = "orgCustomHelpLinkUrl"))]
    pub custom_help_link_url: Option<String>,
    #[serde(rename(deserialize = "orgLegalNotice"))]
    pub legal_notice: Option<String>,
    #[serde(rename(deserialize = "createdAt"), deserialize_with = "date")]
    pub created_at: String,
    #[serde(rename(deserialize = "modifiedAt"), deserialize_with = "date")]
    pub modified_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeamDocument {
    #[serde(rename(deserialize = "_id"))]
    pub team_id: String,
    #[serde(rename(deserialize = "teamDisplayName"))]
    pub display_name: Option<String>,
    #[serde(rename(deserialize = "teamDesc"))]
    pub desc: Option<String>,
    #[serde(rename(deserialize = "teamShortName"))]
    pub short_name: Option<String>,
    #[serde(rename(deserialize = "teamWebsite"))]
    pub website: Option<String>,
    #[serde(rename(deserialize = "teamIsActive"))]
    pub is_active: Option<bool>,
    #[serde(rename(deserialize = "teamSharedTemplates"))]
    pub shared_templates: Option<bool>,
    #[serde(rename(deserialize = "teamPropagateMembersToBoards"))]
    pub propagate_members_to_boards: Option<bool>,
    #[serde(rename(deserialize = "teamSyncMembersFromAuth"))]
    pub sync_members_from_auth: Option<bool>,
    #[serde(rename(deserialize = "createdAt"), deserialize_with = "date")]
    pub created_at: String,
    #[serde(rename(deserialize = "modifiedAt"), deserialize_with = "date")]
    pub modified_at: String,
}

fn date<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let value = String::deserialize(d)?;
    validate_response_date(&value)
        .map_err(|()| serde::de::Error::custom("invalid administrator date-time"))?;
    Ok(value)
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct UpdateOrganizationFeatures {
    #[serde(rename = "orgSharedTemplates", skip_serializing_if = "Option::is_none")]
    pub shared_templates: Option<bool>,
    #[serde(
        rename = "orgPropagateMembersToBoards",
        skip_serializing_if = "Option::is_none"
    )]
    pub propagate_members_to_boards: Option<bool>,
    #[serde(
        rename = "orgSyncMembersFromAuth",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_members_from_auth: Option<bool>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct UpdateTeamFeatures {
    #[serde(
        rename = "teamSharedTemplates",
        skip_serializing_if = "Option::is_none"
    )]
    pub shared_templates: Option<bool>,
    #[serde(
        rename = "teamPropagateMembersToBoards",
        skip_serializing_if = "Option::is_none"
    )]
    pub propagate_members_to_boards: Option<bool>,
    #[serde(
        rename = "teamSyncMembersFromAuth",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_members_from_auth: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureUpdateResult {
    pub updated: u64,
}

#[derive(Serialize)]
struct UpdateAllFeatures {
    field: &'static str,
    value: bool,
}

impl WekanClient {
    pub async fn admin_domains(
        &self,
        token: &SecretString,
    ) -> Result<Vec<DomainCount>, ClientError> {
        self.admin_people_list("domains", token, "admin domain list")
            .await
    }
    pub async fn admin_organizations(
        &self,
        token: &SecretString,
    ) -> Result<Vec<OrganizationSummary>, ClientError> {
        self.admin_people_list("orgs", token, "admin org list")
            .await
    }
    pub async fn admin_teams(&self, token: &SecretString) -> Result<Vec<TeamSummary>, ClientError> {
        self.admin_people_list("teams", token, "admin team list")
            .await
    }
    pub async fn update_organization_features(
        &self,
        id: &str,
        request: &UpdateOrganizationFeatures,
        token: &SecretString,
    ) -> Result<OrganizationDocument, ClientError> {
        let document: OrganizationDocument = self
            .admin_people_update(
                "orgs",
                Some(id),
                request,
                token,
                "admin org features update",
            )
            .await?;
        check_id(&document.organization_id, id)?;
        Ok(document)
    }
    pub async fn update_team_features(
        &self,
        id: &str,
        request: &UpdateTeamFeatures,
        token: &SecretString,
    ) -> Result<TeamDocument, ClientError> {
        let document: TeamDocument = self
            .admin_people_update(
                "teams",
                Some(id),
                request,
                token,
                "admin team features update",
            )
            .await?;
        check_id(&document.team_id, id)?;
        Ok(document)
    }
    pub async fn update_all_organization_features(
        &self,
        field: FeatureField,
        value: bool,
        token: &SecretString,
    ) -> Result<FeatureUpdateResult, ClientError> {
        self.admin_people_update(
            "orgs",
            None,
            &UpdateAllFeatures {
                field: field.org_field(),
                value,
            },
            token,
            "admin org features update-all",
        )
        .await
    }
    pub async fn update_all_team_features(
        &self,
        field: FeatureField,
        value: bool,
        token: &SecretString,
    ) -> Result<FeatureUpdateResult, ClientError> {
        self.admin_people_update(
            "teams",
            None,
            &UpdateAllFeatures {
                field: field.team_field(),
                value,
            },
            token,
            "admin team features update-all",
        )
        .await
    }

    async fn admin_people_list<T: DeserializeOwned>(
        &self,
        resource: &str,
        token: &SecretString,
        operation: &str,
    ) -> Result<Vec<T>, ClientError> {
        let body = self
            .execute_success_request_with_limit(
                self.http
                    .request(
                        Method::GET,
                        self.admin_people_endpoint(resource, None, false)?,
                    )
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
                MAX_RESPONSE_BYTES,
            )
            .await?;
        let entries: Vec<Object<T>> = decode_success(&body, operation, true)?;
        Ok(entries.into_iter().map(|entry| entry.0).collect())
    }
    async fn admin_people_update<T: DeserializeOwned>(
        &self,
        resource: &str,
        id: Option<&str>,
        request: &impl Serialize,
        token: &SecretString,
        operation: &str,
    ) -> Result<T, ClientError> {
        let body = self
            .execute_success_request_with_limit(
                self.http
                    .request(Method::PUT, self.admin_people_endpoint(resource, id, true)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
                MAX_RESPONSE_BYTES,
            )
            .await?;
        decode_success::<Object<T>>(&body, operation, true).map(|entry| entry.0)
    }
    fn admin_people_endpoint(
        &self,
        resource: &str,
        id: Option<&str>,
        features: bool,
    ) -> Result<reqwest::Url, ClientError> {
        if id.is_some_and(|id| id.trim().is_empty() || matches!(id, "." | "..")) {
            return Err(protocol(
                "administrator IDs must be nonempty and not dot-only; no request was sent",
                false,
            ));
        }
        let mut endpoint = self.server().join("api/admin").map_err(|_| {
            protocol(
                "could not construct administrator endpoint; no request was sent",
                false,
            )
        })?;
        let mut segments = endpoint.path_segments_mut().map_err(|()| {
            protocol(
                "could not construct administrator path; no request was sent",
                false,
            )
        })?;
        segments.push(resource);
        if let Some(id) = id {
            segments.push(id);
        }
        if features {
            segments.push("features");
        }
        drop(segments);
        Ok(endpoint)
    }
}

fn check_id(actual: &str, expected: &str) -> Result<(), ClientError> {
    if actual != expected {
        return Err(protocol(
            "Wekan returned a different administrator resource ID than requested",
            true,
        ));
    }
    Ok(())
}
fn protocol(message: &str, success_status_received: bool) -> ClientError {
    ClientError::Protocol {
        message: message.into(),
        diagnostic: None,
        success_status_received,
    }
}
