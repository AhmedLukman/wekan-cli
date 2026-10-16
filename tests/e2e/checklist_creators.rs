// Fixture only: use the same DDP collection inserts as the v11.06 checklist UI.
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use wekan_cli::client::ServerUrl;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

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
        let mut url = ServerUrl::parse(server).unwrap().join("websocket").unwrap();
        assert_eq!(
            url.scheme(),
            "http",
            "DDP fixtures require the isolated HTTP Wekan stack"
        );
        url.set_scheme("ws").unwrap();
        let (mut socket, _) = connect_async(url.as_str())
            .await
            .expect("DDP fixture must connect to the test Wekan stack");
        send(
            &mut socket,
            json!({"msg":"connect", "version":"1", "support":["1"]}),
        )
        .await;
        receive(&mut socket, "connected", None).await;

        let login = method(&mut socket, "1", "login", json!([{"resume":token}])).await;
        let user_id = login["id"]
            .as_str()
            .expect("DDP login must return a user ID")
            .to_owned();
        let checklist_id = format!("uichecklist{nonce}");
        let item_id = format!("uiitem{nonce}");
        method(
            &mut socket,
            "2",
            "/checklists/insert",
            json!([{
                "_id":checklist_id, "cardId":card, "title":"UI checklist", "sort":0,
            }]),
        )
        .await;
        method(
            &mut socket,
            "3",
            "/checklistItems/insert",
            json!([{
                "_id":item_id, "cardId":card, "checklistId":checklist_id,
                "title":"UI item", "sort":0,
            }]),
        )
        .await;
        socket
            .close(None)
            .await
            .expect("DDP fixture socket must close");
        DdpChecklist {
            checklist_id,
            item_id,
            user_id,
        }
    })
    .await
    .expect("DDP checklist fixture timed out")
}

async fn send(socket: &mut Socket, value: Value) {
    socket
        .send(Message::Text(value.to_string().into()))
        .await
        .expect("DDP fixture message must be sent");
}

async fn receive(socket: &mut Socket, kind: &str, id: Option<&str>) -> Value {
    loop {
        let frame = socket
            .next()
            .await
            .expect("DDP fixture connection closed before its response")
            .expect("DDP fixture must receive a WebSocket frame");
        let message = match frame {
            Message::Text(text) => serde_json::from_str::<Value>(&text)
                .expect("DDP fixture must receive a JSON message"),
            Message::Close(_) => panic!("DDP fixture connection closed before its response"),
            _ => continue,
        };
        if message["msg"] == "ping" {
            let mut pong = json!({"msg":"pong"});
            if let Some(id) = message.get("id") {
                pong["id"] = id.clone();
            }
            send(socket, pong).await;
        } else if message["msg"] == "failed" || message["msg"] == "error" {
            panic!("DDP fixture received a protocol error");
        } else if message["msg"] == kind && id.is_none_or(|id| message["id"].as_str() == Some(id)) {
            return message;
        }
    }
}

async fn method(socket: &mut Socket, id: &str, name: &str, params: Value) -> Value {
    send(
        socket,
        json!({"msg":"method", "method":name, "params":params, "id":id}),
    )
    .await;
    let message = receive(socket, "result", Some(id)).await;
    assert!(
        message["error"].is_null(),
        "DDP fixture method {name} failed: {}",
        message["error"]
    );
    message["result"].clone()
}
