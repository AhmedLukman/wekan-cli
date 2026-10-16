//! Test-only transport for fixtures created through Wekan's native UI methods.
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use wekan_cli::client::ServerUrl;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub(crate) struct Session {
    socket: Socket,
    next_id: u64,
    user_id: String,
}

impl Session {
    pub(crate) async fn login(server: &str, token: &str) -> Self {
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
        let mut session = Self {
            socket,
            next_id: 0,
            user_id: String::new(),
        };
        let login = session.call("login", json!([{"resume":token}])).await;
        session.user_id = login["id"]
            .as_str()
            .expect("DDP login must return a user ID")
            .to_owned();
        session
    }

    pub(crate) fn user_id(&self) -> &str {
        &self.user_id
    }

    pub(crate) async fn call(&mut self, name: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id.to_string();
        send(
            &mut self.socket,
            json!({"msg":"method", "method":name, "params":params, "id":id}),
        )
        .await;
        let message = receive(&mut self.socket, "result", Some(&id)).await;
        assert!(
            message["error"].is_null(),
            "DDP fixture method {name} failed: {}",
            message["error"]
        );
        message["result"].clone()
    }

    pub(crate) async fn close(mut self) {
        self.socket
            .close(None)
            .await
            .expect("DDP fixture socket must close");
    }
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
