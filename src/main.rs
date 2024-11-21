mod chat;
mod chat_listener;
mod gui;
mod latest_chat_messages;
mod logic;
mod prize_sender;
mod registered_viewers;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use chat_listener::listen_to_chat;
use gui::run_gui;
use logic::RaffleLogic;
use prize_sender::PrizeSender;
use rand::{thread_rng, RngCore};
use rsnano_core::{Amount, RawKey};
use rsnano_nullable_clock::SteadyClock;
use tokio::time::sleep;

fn main() -> eframe::Result {
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
        let winner = logic
            .lock()
            .unwrap()
            .tick(clock.now(), thread_rng().next_u32());
        if let Some(winner) = winner {
            println!(
                "WE HAVE A WINNER: {} with address {}",
                winner.name,
                winner.account.encode_account()
            );
            std::process::Command::new("notify-send")
                .arg("-i")
                .arg("face-smile-big-symbolic")
                .arg(format!(
                    "Congratulations {}! You've just won Ӿ {}",
                    winner.name,
                    winner.amount.format_balance(1)
                ))
                .output()
                .unwrap();

            prize_sender.send_prize(winner.account, winner.amount).await;
        }
        sleep(Duration::from_secs(1)).await
    }
}
