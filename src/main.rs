mod chat;
mod chat_listener;
mod gui;
mod latest_chat_messages;
mod logic;
mod prize_sender;
mod registered_viewers;

use std::{
    ffi::OsStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use chat_listener::listen_to_chat;
use gui::run_gui;
use log::{info, warn};
use logic::{OutputAction, RaffleLogic};
use prize_sender::PrizeSender;
use rand::{thread_rng, RngCore};
use rsnano_core::RawKey;
use rsnano_nullable_clock::SteadyClock;
use tokio::{process::Command, time::sleep};

fn main() -> eframe::Result {
    env_logger::init();
    let stream_url = std::env::var("STREAM_URL").unwrap();
    let priv_key = std::env::var("NANO_PRV_KEY").unwrap();
    let priv_key = RawKey::decode_hex(priv_key).unwrap();

    let logic = Arc::new(Mutex::new(RaffleLogic::default()));
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
        rt.spawn(async move { run_ticker(logic2.clone(), clock, priv_key).await });

        rt.block_on(async move {
            listen_to_chat(stream_url, move |msg| {
                logic.lock().unwrap().handle_chat_message(msg)
            })
            .await;
        });
    });
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
                OutputAction::Notify(message) => {
                    show_notification(message).await;
                }
                OutputAction::SendToWinner(winner) => {
                    info!(
                        "We have a winner: {} with address {}",
                        winner.name,
                        winner.account.encode_account()
                    );

                    match prize_sender.send_prize(winner.account, winner.amount).await {
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
