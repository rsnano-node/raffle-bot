use crate::{
    chat_messages::ChatMessage,
    http_server::run_http_server,
    logic::{Action, RaffleLogic},
    prize_sender::PrizeSender,
    twitch_chat_listener::listen_to_twitch_chat,
    youtube_chat_listener::listen_to_youtube_chat,
};
use log::{info, warn};
use rand::{thread_rng, RngCore};
use rsnano_core::PrivateKey;
use rsnano_nullable_clock::SteadyClock;
use std::{
    ffi::OsStr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{process::Command, sync::oneshot::Receiver, task::JoinSet, time::sleep};

pub(crate) fn run_backend(
    logic: &Arc<Mutex<RaffleLogic>>,
    clock: &Arc<SteadyClock>,
    priv_key: PrivateKey,
    stop: Receiver<()>,
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let logic_l = logic.clone();
    let handle_message = move |msg: ChatMessage| logic_l.lock().unwrap().handle_chat_message(msg);
    let handle_message2 = handle_message.clone();

    runtime.block_on(async {
        let mut set = JoinSet::new();
        let logic_l = logic.clone();
        let clock_l = clock.clone();
        set.spawn(async move { run_ticker(&logic_l, &clock_l, priv_key).await });
        set.spawn(run_http_server(logic.clone(), clock.clone()));
        set.spawn(listen_to_twitch_chat(handle_message));
        set.spawn(listen_to_youtube_chat(handle_message2));

        tokio::select!(
            _ = set.join_all() => {},
            _ = stop =>{}
        );
    });
}

async fn run_ticker(logic: &Mutex<RaffleLogic>, clock: &SteadyClock, priv_key: PrivateKey) {
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
