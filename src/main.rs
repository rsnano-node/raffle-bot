mod backend;
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
    sync::{Arc, Mutex},
    time::Duration,
};

use backend::run_backend;
use gui::run_gui;
use log::info;
use logic::RaffleLogic;
use rsnano_core::{Amount, PublicKey, RawKey};
use rsnano_nullable_clock::SteadyClock;
use tokio::sync::oneshot::{self};

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
