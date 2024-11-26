mod chat;
mod chat_listener;
mod gui;
mod logic;
mod participants;
mod prize_sender;
mod raffle_runner;
mod upcoming_raffle_announcement;

use std::{
    ffi::OsStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    extract::State,
    http::{header, HeaderMap},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use chat_listener::listen_to_chat;
use gui::run_gui;
use log::{info, warn};
use logic::{Action, RaffleLogic};
use prize_sender::PrizeSender;
use rand::{thread_rng, RngCore};
use rsnano_core::{Amount, PublicKey, RawKey};
use rsnano_nullable_clock::SteadyClock;
use serde::Serialize;
use tokio::{net::TcpListener, process::Command, time::sleep};

fn main() -> eframe::Result {
    env_logger::init();
    let stream_url = std::env::var("STREAM_URL").unwrap();
    info!("using stream url: {}", stream_url);
    let priv_key = std::env::var("NANO_PRV_KEY").unwrap();
    let priv_key = RawKey::decode_hex(priv_key).unwrap();
    info!(
        "using account: {}",
        PublicKey::try_from(&priv_key)
            .unwrap()
            .as_account()
            .encode_account()
    );
    let prize = std::env::var("NANO_PRIZE")
        .ok()
        .map(|s| Amount::decode_dec(s).unwrap());
    let interval = std::env::var("RAFFLE_INTERVAL")
        .ok()
        .map(|s| s.parse::<u64>().unwrap());
    let mut logic = RaffleLogic::default();
    if let Some(prize) = prize {
        info!("using prize of {}", prize.format_balance(2));
        logic.set_prize(prize);
    }
    if let Some(interval) = interval {
        info!("using interval of {}s", interval);
        logic.set_raffle_interval(Duration::from_secs(interval));
    }
    let logic = Arc::new(Mutex::new(logic));
    let clock = Arc::new(SteadyClock::default());
    spawn_backend(logic.clone(), clock.clone(), stream_url, priv_key);
    run_gui(logic, clock)
}

fn spawn_backend(
    logic: Arc<Mutex<RaffleLogic>>,
    clock: Arc<SteadyClock>,
    stream_url: String,
    priv_key: RawKey,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let logic2 = logic.clone();
        let logic3 = logic.clone();
        let clock2 = clock.clone();
        rt.spawn(async move { run_ticker(logic2, clock, priv_key).await });

        rt.spawn(run_http_server(logic3, clock2));

        rt.block_on(async move {
            listen_to_chat(stream_url, move |msg| {
                logic.lock().unwrap().handle_chat_message(msg)
            })
            .await;
        });
    });
}

async fn run_http_server(logic: Arc<Mutex<RaffleLogic>>, clock: Arc<SteadyClock>) {
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

async fn run_ticker(logic: Arc<Mutex<RaffleLogic>>, clock: Arc<SteadyClock>, priv_key: RawKey) {
    let prize_sender = PrizeSender::new(priv_key);
    loop {
        let actions = logic
            .lock()
            .unwrap()
            .tick(clock.now(), thread_rng().next_u32());

        for action in actions {
            match action {
                Action::Notify(message) => {
                    show_notification(message).await;
                }
                Action::SendToWinner(winner) => {
                    info!(
                        "We have a winner: {} with address {}",
                        winner.name,
                        winner.account.encode_account()
                    );

                    match prize_sender.send_prize(winner.account, winner.prize).await {
                        Ok(_) => info!("Prize sent!"),
                        Err(e) => warn!("Could not send prize: {:?}", e),
                    }
                }
            }
        }
        sleep(Duration::from_secs(1)).await
    }
}

async fn show_notification(message: impl AsRef<OsStr>) {
    match Command::new("notify-send")
        .arg("-i")
        .arg("face-smile-big-symbolic")
        .arg(message)
        .output()
        .await
    {
        Ok(_) => {}
        Err(e) => warn!("Could not send notification: {:?}", e),
    }
}
