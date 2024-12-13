use axum::{
    extract::State,
    http::{header, HeaderMap},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use rsnano_nullable_clock::SteadyClock;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

use crate::logic::RaffleLogic;

pub(crate) async fn run_http_server(logic: Arc<Mutex<RaffleLogic>>, clock: Arc<SteadyClock>) {
    let app = Router::new()
        .route("/", get(get_html))
        .route("/raffle", get(get_raffle))
        .route("/confirm", post(post_confirm))
        .route("/overlay.svg", get(get_overlay))
        .with_state((logic, clock));
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_html() -> Html<&'static str> {
    Html(include_str!("../assets/index.html"))
}

async fn get_overlay() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/svg+xml".parse().unwrap());
    (headers, include_str!("../assets/overlay.svg"))
}

#[derive(Serialize)]
struct SpinInstruction {
    spin: bool,
    participants: Vec<String>,
    winner: usize,
}

async fn get_raffle(
    State((logic, clock)): State<(Arc<Mutex<RaffleLogic>>, Arc<SteadyClock>)>,
) -> Json<SpinInstruction> {
    let mut guard = logic.lock().unwrap();
    guard.ping(clock.now());
    if let Some(win) = guard.current_win() {
        let participants = win.participants.clone();
        let winner = participants.iter().position(|i| i == &win.winner).unwrap();
        Json(SpinInstruction {
            spin: true,
            participants,
            winner,
        })
    } else {
        Json(SpinInstruction {
            spin: false,
            participants: Vec::new(),
            winner: 0,
        })
    }
}

async fn post_confirm(State((logic, _)): State<(Arc<Mutex<RaffleLogic>>, Arc<SteadyClock>)>) {
    let mut guard = logic.lock().unwrap();
    guard.spin_finished();
}
