use serde_json::{Value, json};

pub fn organization() -> Value {
    json!({
        "_id":"org-1", "orgDisplayName":"Organization\u{1b}[31m", "orgDesc":"Description", "orgShortName":"org",
        "orgAutoAddUsersWithDomainName":"example.test", "orgWebsite":"https://example.test", "orgIsActive":true,
        "orgSharedTemplates":false, "orgPropagateMembersToBoards":true, "orgSyncMembersFromAuth":false,
        "orgDomains":"tenant.example.test", "orgProductName":"Tenant", "orgThemeColor":"board-color-nephritis",
        "orgThemeCustomColors":["#112233",null], "orgCustomLoginLogoImageUrl":"/logo.png", "orgCustomLoginLogoLinkUrl":"/home",
        "orgTextBelowCustomLoginLogo":"Welcome", "orgCustomTopLeftCornerLogoImageUrl":"/corner.png",
        "orgCustomTopLeftCornerLogoLinkUrl":"/", "orgCustomHelpLinkUrl":"/help", "orgLegalNotice":"Terms",
        "createdAt":"2026-10-15T00:00:00.000Z", "modifiedAt":"2026-10-15T01:00:00.000Z"
    })
}
pub fn team() -> Value {
    json!({
        "_id":"team-1", "teamDisplayName":"Team\u{1b}[31m", "teamDesc":"Description", "teamShortName":"team",
        "teamWebsite":"https://example.test", "teamIsActive":true,
        "teamSharedTemplates":false, "teamPropagateMembersToBoards":true, "teamSyncMembersFromAuth":false,
        "createdAt":"2026-10-15T00:00:00.000Z", "modifiedAt":"2026-10-15T01:00:00.000Z"
    })
}
pub fn summary(prefix: &str, document: &Value) -> Value {
    let mut result = json!({"_id": document["_id"]});
    for suffix in [
        "DisplayName",
        "ShortName",
        "SharedTemplates",
        "PropagateMembersToBoards",
        "SyncMembersFromAuth",
    ] {
        let key = format!("{prefix}{suffix}");
        if let Some(value) = document.get(&key) {
            result[key] = value.clone();
        }
    }
    result
}
pub const FEATURES: [(&str, &str, &str); 3] = [
    ("SharedTemplates", "shared-templates", "shared_templates"),
    (
        "PropagateMembersToBoards",
        "propagate-members-to-boards",
        "propagate_members_to_boards",
    ),
    (
        "SyncMembersFromAuth",
        "sync-members-from-auth",
        "sync_members_from_auth",
    ),
];
