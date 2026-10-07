pub mod protocol;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};
use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    response::{Html, IntoResponse},
    routing::get,
};

use protocol::{ClientMsg, PhoneInput, PlayerInfo, ServerMsg};

const PHONE_HTML: &str = include_str!("../../assets/phone.html");

#[derive(Clone)]
struct Shared(Arc<Inner>);

struct Inner {
    next_id: AtomicU32,
    players: Mutex<Vec<PlayerInfo>>,
    state_tx: broadcast::Sender<String>,
    input_tx: mpsc::UnboundedSender<PhoneInput>,
}

pub struct GameServer {
    inner: Arc<Inner>,
    input_rx: Mutex<mpsc::UnboundedReceiver<PhoneInput>>,
    pub port: u16,
    pub local_ip: String,
}

impl GameServer {
    pub fn start(port: u16, rt: &tokio::runtime::Runtime) -> Arc<Self> {
        let (state_tx, _) = broadcast::channel::<String>(512);
        let (input_tx, input_rx) = mpsc::unbounded_channel::<PhoneInput>();

        let inner = Arc::new(Inner {
            next_id: AtomicU32::new(1),
            players: Mutex::new(Vec::new()),
            state_tx,
            input_tx,
        });

        let local_ip = Self::detect_local_ip();

        let server = Arc::new(GameServer {
            inner: Arc::clone(&inner),
            input_rx: Mutex::new(input_rx),
            port,
            local_ip,
        });

        let shared = Shared(Arc::clone(&inner));
        let router = Router::new()
            .route("/", get(serve_phone_html))
            .route("/ws", get(ws_upgrade))
            .with_state(shared);

        rt.spawn(async move {
            let addr = format!("0.0.0.0:{}", port);
            let listener = tokio::net::TcpListener::bind(&addr)
                .await
                .expect("could not bind multiplayer server");
            axum::serve(listener, router)
                .await
                .expect("multiplayer server error");
        });

        server
    }

    pub fn broadcast(&self, msg: &ServerMsg) {
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = self.inner.state_tx.send(json);
        }
    }

    pub fn pop_input(&self) -> Option<PhoneInput> {
        self.input_rx.lock().ok()?.try_recv().ok()
    }

    pub fn players(&self) -> Vec<PlayerInfo> {
        self.inner.players.lock().unwrap().clone()
    }

    pub fn player_name(&self, id: u32) -> Option<String> {
        self.inner
            .players
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone())
    }

    pub fn add_score(&self, player_id: u32, delta: i32) {
        if let Ok(mut players) = self.inner.players.lock() {
            if let Some(p) = players.iter_mut().find(|p| p.id == player_id) {
                p.score += delta;
            }
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}:{}", self.local_ip, self.port)
    }

    fn detect_local_ip() -> String {
        use std::net::UdpSocket;
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    return addr.ip().to_string();
                }
            }
        }
        "127.0.0.1".to_string()
    }
}

async fn serve_phone_html() -> Html<&'static str> {
    Html(PHONE_HTML)
}

async fn ws_upgrade(
    upgrade: WebSocketUpgrade,
    State(shared): State<Shared>,
) -> impl IntoResponse {
    let player_id = shared.0.next_id.fetch_add(1, Ordering::SeqCst);
    upgrade.on_upgrade(move |socket| handle_socket(socket, shared, player_id))
}

async fn handle_socket(mut socket: WebSocket, shared: Shared, player_id: u32) {
    let mut rx = shared.0.state_tx.subscribe();

    // Send initial lobby state so phone knows its own ID and current player list
    {
        let players = shared.0.players.lock().unwrap().clone();
        let msg = ServerMsg::Lobby { players, you: player_id };
        if let Ok(json) = serde_json::to_string(&msg) {
            if socket.send(Message::Text(json)).await.is_err() {
                return;
            }
        }
    }

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) {
                            if let ClientMsg::Join { ref name } = client_msg {
                                {
                                    let mut players = shared.0.players.lock().unwrap();
                                    if !players.iter().any(|p| p.id == player_id) {
                                        players.push(PlayerInfo {
                                            id: player_id,
                                            name: name.clone(),
                                            score: 0,
                                        });
                                    }
                                }
                                // Broadcast updated lobby to all connected clients
                                let players = shared.0.players.lock().unwrap().clone();
                                let lobby = ServerMsg::Lobby { players, you: player_id };
                                if let Ok(json) = serde_json::to_string(&lobby) {
                                    let _ = shared.0.state_tx.send(json);
                                }
                            }
                            let _ = shared.0.input_tx.send(PhoneInput { player_id, msg: client_msg });
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            outgoing = rx.recv() => {
                match outgoing {
                    Ok(json) => {
                        if socket.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                }
            }
        }
    }

    // Clean up: remove from player list and notify remaining clients
    shared.0.players.lock().unwrap().retain(|p| p.id != player_id);
    let players = shared.0.players.lock().unwrap().clone();
    let lobby = ServerMsg::Lobby { players, you: 0 };
    if let Ok(json) = serde_json::to_string(&lobby) {
        let _ = shared.0.state_tx.send(json);
    }
}
