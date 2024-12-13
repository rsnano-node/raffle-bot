mod backend;
mod chat_messages;
mod gui;
mod http_server;
mod logic;
mod participants;
mod participants_file;
mod prize_sender;
mod raffle_runner;
mod twitch_chat_listener;
mod youtube_chat_listener;

use std::{
    env,
    sync::{Arc, Mutex},
    time::Duration,
};

use backend::run_backend;
use gui::run_gui;
use log::info;
use logic::RaffleLogic;
use participants_file::ParticipantsFile;
use rsnano_core::{Amount, PrivateKey};
use rsnano_nullable_clock::SteadyClock;
use tokio::sync::oneshot::{self};

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "raffle_bot=debug")
    }
    env_logger::init();
    let priv_key = std::env::var("NANO_PRV_KEY").expect("env var NANO_PRV_KEY not set!");
    let priv_key = PrivateKey::from_hex_str(priv_key).unwrap();
    info!("using account: {}", priv_key.account().encode_account());
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
    let mut participants_file = ParticipantsFile::default();
    logic.set_participants(participants_file.load());
    let logic = Arc::new(Mutex::new(logic));
    let clock = Arc::new(SteadyClock::default());
    let (tx_stop, rx_stop) = oneshot::channel::<()>();

    std::thread::scope(|s| {
        s.spawn(|| run_backend(&logic, &clock, priv_key, participants_file, rx_stop));
        run_gui(logic.clone(), clock.clone()).unwrap();
        tx_stop.send(()).unwrap();
    })
}
