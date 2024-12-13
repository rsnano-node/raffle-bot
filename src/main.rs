mod chat;
mod gui;
mod http_server;
mod logic;
mod participants;
mod prize_sender;
mod raffle_runner;
mod twitch_chat_listener;
mod youtube_chat_listener;

use std::{
    ffi::OsStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use gui::run_gui;
use http_server::run_http_server;
use log::{info, warn};
use logic::{Action, RaffleLogic};
use prize_sender::PrizeSender;
use rand::{thread_rng, RngCore};
use rsnano_core::{Amount, PublicKey, RawKey};
use rsnano_nullable_clock::SteadyClock;
use tokio::{
    process::Command,
    sync::oneshot::{self, Receiver},
    time::sleep,
};
use twitch_chat_listener::listen_to_twitch_chat;
use youtube_chat_listener::listen_to_youtube_chat;

fn main() {
    env_logger::init();
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
    let (tx_stop, rx_stop) = oneshot::channel::<()>();

    std::thread::scope(|s| {
        s.spawn(|| run_backend(&logic, &clock, priv_key, rx_stop));
        run_gui(logic.clone(), clock.clone()).unwrap();
        tx_stop.send(()).unwrap();
    })
}

fn run_backend(
    logic: &Arc<Mutex<RaffleLogic>>,
    clock: &Arc<SteadyClock>,
    priv_key: RawKey,
    stop: Receiver<()>,
) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        tokio::select!(
            _ = run_ticker(logic, clock, priv_key) => {},
            _ = run_http_server(logic.clone(), clock.clone()) => {},
            _ = listen_to_twitch_chat(|msg| {
                logic.lock().unwrap().handle_chat_message(msg)
            }) => {}
            _ = listen_to_youtube_chat(|msg| {
                logic.lock().unwrap().handle_chat_message(msg)
            }) => {}
            _ = stop =>{}
        );
    });
}

async fn run_ticker(logic: &Mutex<RaffleLogic>, clock: &SteadyClock, priv_key: RawKey) {
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
