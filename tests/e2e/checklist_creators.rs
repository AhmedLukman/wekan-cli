// Fixture only: use the same DDP collection inserts as the v12.57 checklist UI.
use std::time::Duration;

use serde_json::json;

use crate::support::ddp::Session;

pub(super) struct DdpChecklist {
    pub(super) checklist_id: String,
    pub(super) item_id: String,
    pub(super) user_id: String,
}

pub(super) async fn create_ui_checklist(
    server: &str,
    card: &str,
    token: &str,
    nonce: u128,
) -> DdpChecklist {
    tokio::time::timeout(Duration::from_secs(30), async {
        let mut session = Session::login(server, token).await;
        let user_id = session.user_id().to_owned();
        let checklist_id = format!("uichecklist{nonce}");
        let item_id = format!("uiitem{nonce}");
        session
            .call(
                "/checklists/insert",
                json!([{
                    "_id":checklist_id, "cardId":card, "title":"UI checklist", "sort":0,
                }]),
            )
            .await;
        session
            .call(
                "/checklistItems/insert",
                json!([{
                    "_id":item_id, "cardId":card, "checklistId":checklist_id,
                    "title":"UI item", "sort":0,
                }]),
            )
            .await;
        session.close().await;
        DdpChecklist {
            checklist_id,
            item_id,
            user_id,
        }
    })
    .await
    .expect("DDP checklist fixture timed out")
}
