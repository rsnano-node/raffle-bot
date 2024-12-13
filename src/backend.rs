use crate::{
    http_server::run_http_server,
    logic::{Action, RaffleLogic},
    prize_sender::PrizeSender,
    twitch_chat_listener::listen_to_twitch_chat,
    youtube_chat_listener::listen_to_youtube_chat,
};
use log::{info, warn};
use rand::{thread_rng, RngCore};
use rsnano_core::RawKey;
use rsnano_nullable_clock::SteadyClock;
use std::{
    ffi::OsStr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{process::Command, sync::oneshot::Receiver, time::sleep};

pub(crate) fn run_backend(
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
