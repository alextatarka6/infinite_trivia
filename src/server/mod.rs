pub mod protocol;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::mpsc;
use axum::{
    Router,
    extract::{Path, State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    http::{header, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
};

use protocol::{ClientMsg, PhoneInput, PlayerInfo, ServerMsg};

const PHONE_HTML: &str = include_str!("../../assets/phone.html");

/// Fonts the phone page loads from the host, so it renders on-brand without internet.
const PHONE_FONTS: &[(&str, &[u8])] = &[
    ("BarlowCondensed-Bold.ttf", include_bytes!("../../assets/fonts/BarlowCondensed-Bold.ttf")),
    ("BarlowCondensed-ExtraBold.ttf", include_bytes!("../../assets/fonts/BarlowCondensed-ExtraBold.ttf")),
    ("Barlow-Medium.ttf", include_bytes!("../../assets/fonts/Barlow-Medium.ttf")),
];

#[derive(Clone)]
struct Shared(Arc<Inner>);

/// A seat survives disconnects: phones reclaim it with their persisted token,
/// so a screen lock or Wi-Fi blip doesn't drop the player from the game.
struct Seat {
    info: PlayerInfo,
    token: String,
    /// Live connection, if any. Replaced when the phone reconnects.
    conn: Option<(u64, mpsc::UnboundedSender<String>)>,
    /// Last game-state message this player was sent, replayed on reconnect.
    last_state: Option<String>,
}

struct Inner {
    next_id: AtomicU32,
    next_conn: AtomicU32,
    seats: Mutex<Vec<Seat>>,
    /// Connections that have not claimed a seat yet (still on the name form).
    pending: Mutex<HashMap<u64, mpsc::UnboundedSender<String>>>,
    input_tx: mpsc::UnboundedSender<PhoneInput>,
    /// The host UI, woken when phone input or roster changes arrive so it
    /// doesn't have to redraw continuously to notice them.
    repaint: Mutex<Option<egui::Context>>,
    /// Debug builds: every raw client message, for the phone test harness.
    #[cfg(debug_assertions)]
    dev_log: Mutex<Vec<String>>,
}

/// Sentinel queued on a connection's channel to make `handle_socket` hang up.
const CLOSE_SENTINEL: &str = "\0close";

pub struct GameServer {
    inner: Arc<Inner>,
    input_rx: Mutex<mpsc::UnboundedReceiver<PhoneInput>>,
    pub port: u16,
    pub local_ip: String,
}

impl GameServer {
    /// Bind the first free port in `port..port + 10` and start serving.
    /// Errors (every port taken, no permission) come back to the caller.
    pub fn start(port: u16, rt: &tokio::runtime::Runtime) -> Result<Arc<Self>, String> {
        let (input_tx, input_rx) = mpsc::unbounded_channel::<PhoneInput>();

        let inner = Arc::new(Inner {
            next_id: AtomicU32::new(1),
            next_conn: AtomicU32::new(1),
            seats: Mutex::new(Vec::new()),
            pending: Mutex::new(HashMap::new()),
            input_tx,
            repaint: Mutex::new(None),
            #[cfg(debug_assertions)]
            dev_log: Mutex::new(Vec::new()),
        });

        let local_ip = Self::detect_local_ip();

        let mut bound = None;
        let mut last_err = String::new();
        for p in port..port.saturating_add(10) {
            match rt.block_on(tokio::net::TcpListener::bind(("0.0.0.0", p))) {
                Ok(l) => {
                    bound = Some((p, l));
                    break;
                }
                Err(e) => last_err = e.to_string(),
            }
        }
        let Some((port, listener)) = bound else {
            return Err(format!("Could not open a port for phones ({}–{}): {last_err}", port, port.saturating_add(9)));
        };

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
            .route("/fonts/:name", get(serve_font));
        #[cfg(debug_assertions)]
        let router = dev::routes(router);
        let router = router.with_state(shared);

        rt.spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                eprintln!("multiplayer server stopped: {e}");
            }
        });

        Ok(server)
    }

    /// Wake `ctx` whenever phone input or roster changes arrive.
    pub fn set_repaint_ctx(&self, ctx: &egui::Context) {
        *self.inner.repaint.lock().unwrap() = Some(ctx.clone());
    }

    /// True when phones on the network can reach us (we found a LAN address).
    pub fn lan_reachable(&self) -> bool {
        self.local_ip != "127.0.0.1"
    }

    /// Send the same game-state message to every seated player.
    pub fn broadcast(&self, msg: &ServerMsg) {
        self.send_each(|_| msg.clone());
    }

    /// Send a per-player message (e.g. `is_your_turn`) to every seated player.
    pub fn send_each(&self, mut msg_for: impl FnMut(u32) -> ServerMsg) {
        let mut seats = self.inner.seats.lock().unwrap();
        for seat in seats.iter_mut() {
            if let Ok(json) = serde_json::to_string(&msg_for(seat.info.id)) {
                deliver(seat, json, true);
            }
        }
    }

    pub fn pop_input(&self) -> Option<PhoneInput> {
        self.input_rx.lock().ok()?.try_recv().ok()
    }

    pub fn players(&self) -> Vec<PlayerInfo> {
        self.inner.seats.lock().unwrap().iter().map(|s| s.info.clone()).collect()
    }

    pub fn player_name(&self, id: u32) -> Option<String> {
        self.inner
            .seats
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.info.id == id)
            .map(|s| s.info.name.clone())
    }

    pub fn add_score(&self, player_id: u32, delta: i32) {
        if let Ok(mut seats) = self.inner.seats.lock() {
            if let Some(s) = seats.iter_mut().find(|s| s.info.id == player_id) {
                s.info.score += delta;
            }
        }
    }

    /// Mirror a score kept elsewhere (the Jeopardy board) onto a seat.
    pub fn set_score(&self, player_id: u32, score: i32) {
        if let Ok(mut seats) = self.inner.seats.lock() {
            if let Some(s) = seats.iter_mut().find(|s| s.info.id == player_id) {
                s.info.score = score;
            }
        }
    }

    /// Zero every seat's score: called when a new game starts.
    pub fn reset_scores(&self) {
        if let Ok(mut seats) = self.inner.seats.lock() {
            for s in seats.iter_mut() {
                s.info.score = 0;
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

/// Queue `json` on the seat's live connection; game-state messages are also
/// remembered so a reconnecting phone lands back on the right screen.
fn deliver(seat: &mut Seat, json: String, remember: bool) {
    if let Some((_, tx)) = &seat.conn {
        let _ = tx.send(json.clone());
    }
    if remember {
        seat.last_state = Some(json);
    }
}

fn wake(inner: &Inner) {
    if let Some(ctx) = inner.repaint.lock().unwrap().as_ref() {
        ctx.request_repaint();
    }
}

/// Push the current roster to every connection (seated or still naming).
fn broadcast_lobby(inner: &Inner) {
    wake(inner);
    let seats = inner.seats.lock().unwrap();
    let players: Vec<PlayerInfo> = seats.iter().map(|s| s.info.clone()).collect();
    let Ok(json) = serde_json::to_string(&ServerMsg::Lobby { players }) else { return };
    for seat in seats.iter() {
        if let Some((_, tx)) = &seat.conn {
            let _ = tx.send(json.clone());
        }
    }
    for tx in inner.pending.lock().unwrap().values() {
        let _ = tx.send(json.clone());
    }
}

async fn serve_phone_html() -> Html<&'static str> {
    Html(PHONE_HTML)
}

async fn serve_font(Path(name): Path<String>) -> impl IntoResponse {
    match PHONE_FONTS.iter().find(|(n, _)| *n == name) {
        Some((_, bytes)) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "font/ttf"), (header::CACHE_CONTROL, "max-age=86400")],
            *bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn ws_upgrade(
    upgrade: WebSocketUpgrade,
    State(shared): State<Shared>,
) -> impl IntoResponse {
    upgrade.on_upgrade(move |socket| handle_socket(socket, shared))
}

/// Bind connection `conn` to the seat for `token`, creating it if `name` is given.
/// Returns the seat's player id, or None if the token is unknown and no name was sent.
fn claim_seat(
    inner: &Inner,
    conn: u64,
    tx: &mpsc::UnboundedSender<String>,
    token: &str,
    name: Option<&str>,
) -> Option<u32> {
    let mut seats = inner.seats.lock().unwrap();
    let idx = match seats.iter().position(|s| s.token == token) {
        Some(i) => i,
        None => {
            let name = name?.trim();
            if name.is_empty() {
                return None;
            }
            seats.push(Seat {
                info: PlayerInfo {
                    id: inner.next_id.fetch_add(1, Ordering::SeqCst),
                    name: name.chars().take(20).collect(),
                    score: 0,
                    connected: true,
                },
                token: token.to_string(),
                conn: None,
                last_state: None,
            });
            seats.len() - 1
        }
    };
    let seat = &mut seats[idx];
    seat.conn = Some((conn, tx.clone()));
    seat.info.connected = true;
    let joined = ServerMsg::Joined { you: seat.info.id, name: seat.info.name.clone() };
    if let Ok(json) = serde_json::to_string(&joined) {
        let _ = tx.send(json);
    }
    if let Some(state) = &seat.last_state {
        let _ = tx.send(state.clone());
    }
    inner.pending.lock().unwrap().remove(&conn);
    Some(seat.info.id)
}

async fn handle_socket(mut socket: WebSocket, shared: Shared) {
    let inner = &shared.0;
    let conn = inner.next_conn.fetch_add(1, Ordering::SeqCst) as u64;
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    inner.pending.lock().unwrap().insert(conn, tx.clone());
    let mut player_id: Option<u32> = None;

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let text = match incoming {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => continue,
                };
                #[cfg(debug_assertions)]
                inner.dev_log.lock().unwrap().push(text.clone());
                let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) else { continue };
                match &client_msg {
                    ClientMsg::Hello { token } => {
                        player_id = claim_seat(inner, conn, &tx, token, None);
                        if player_id.is_none() {
                            if let Ok(json) = serde_json::to_string(&ServerMsg::NeedName) {
                                let _ = tx.send(json);
                            }
                        }
                        broadcast_lobby(inner);
                    }
                    ClientMsg::Join { name, token } => {
                        player_id = claim_seat(inner, conn, &tx, token, Some(name));
                        broadcast_lobby(inner);
                    }
                    _ => {}
                }
                if let Some(id) = player_id {
                    let _ = inner.input_tx.send(PhoneInput { player_id: id, msg: client_msg });
                    wake(inner);
                }
            }
            outgoing = rx.recv() => {
                let Some(json) = outgoing else { break };
                if json == CLOSE_SENTINEL {
                    break;
                }
                if socket.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    }

    // Keep the seat (the phone may be locked or switching networks); just mark it offline.
    inner.pending.lock().unwrap().remove(&conn);
    {
        let mut seats = inner.seats.lock().unwrap();
        if let Some(seat) = seats.iter_mut().find(|s| matches!(s.conn, Some((c, _)) if c == conn)) {
            seat.conn = None;
            seat.info.connected = false;
        }
    }
    broadcast_lobby(inner);
}

/// Debug-only HTTP hooks used by the phone test harness (`IT_PHONE_TEST=1`).
///   POST /dev/push[?to=ID]  raw ServerMsg JSON -> every seat (or one), remembered for replay
///   POST /dev/drop          hang up every connection (simulates a phone screen lock)
///   GET  /dev/state         seats + every raw client message received
#[cfg(debug_assertions)]
mod dev {
    use super::*;
    use axum::{extract::Query, routing::post, Json};

    pub fn routes(router: Router<Shared>) -> Router<Shared> {
        router
            .route("/dev/push", post(push))
            .route("/dev/drop", post(drop_all))
            .route("/dev/state", get(state))
    }

    #[derive(serde::Deserialize)]
    struct To {
        to: Option<u32>,
    }

    async fn push(State(shared): State<Shared>, Query(q): Query<To>, body: String) -> &'static str {
        let mut seats = shared.0.seats.lock().unwrap();
        for seat in seats.iter_mut().filter(|s| q.to.is_none_or(|id| id == s.info.id)) {
            deliver(seat, body.clone(), true);
        }
        "ok"
    }

    async fn drop_all(State(shared): State<Shared>) -> &'static str {
        for seat in shared.0.seats.lock().unwrap().iter() {
            if let Some((_, tx)) = &seat.conn {
                let _ = tx.send(CLOSE_SENTINEL.to_string());
            }
        }
        for tx in shared.0.pending.lock().unwrap().values() {
            let _ = tx.send(CLOSE_SENTINEL.to_string());
        }
        "ok"
    }

    async fn state(State(shared): State<Shared>) -> Json<serde_json::Value> {
        let players: Vec<PlayerInfo> =
            shared.0.seats.lock().unwrap().iter().map(|s| s.info.clone()).collect();
        let log = shared.0.dev_log.lock().unwrap().clone();
        Json(serde_json::json!({ "players": players, "log": log }))
    }
}
