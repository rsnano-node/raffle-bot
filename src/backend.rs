use crate::{
    chat_messages::ChatMessage,
    http_server::run_http_server,
    logic::{Action, RaffleLogic},
    participants_file::ParticipantsFile,
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
    participants_file: ParticipantsFile,
    stop: Receiver<()>,
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let logic_l = logic.clone();
    let handle_message = move |msg: ChatMessage| logic_l.lock().unwrap().handle_chat_message(msg);

    runtime.block_on(async {
        let mut set = JoinSet::new();
        set.spawn(run_ticker(
            logic.clone(),
            clock.clone(),
            participants_file,
            priv_key,
        ));
        set.spawn(run_http_server(logic.clone(), clock.clone()));
        set.spawn(listen_to_twitch_chat(handle_message.clone()));
        set.spawn(listen_to_youtube_chat(handle_message));

        tokio::select!(
            _ = set.join_all() => {},
            _ = stop =>{}
        );
    });
}

/// Periodically check logic for new things to do
async fn run_ticker(
    logic: Arc<Mutex<RaffleLogic>>,
    clock: Arc<SteadyClock>,
    mut participants_file: ParticipantsFile,
    priv_key: PrivateKey,
) {
    let prize_sender = PrizeSender::new(priv_key);
    loop {
        let participants;
        let actions;
        {
            let mut guard = logic.lock().unwrap();
            participants = guard.participants();
            actions = guard.tick(clock.now(), thread_rng().next_u32())
        };

        participants_file.update(participants);

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
