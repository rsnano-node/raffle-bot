use crate::participants::ParticipantRegistry;
use rsnano_core::{Account, Amount};
use rsnano_nullable_clock::Timestamp;
use std::time::Duration;

pub(crate) struct RaffleRunner {
    next_raffle: Option<Timestamp>,
    prize: Amount,
    interval: Duration,
}

impl Default for RaffleRunner {
    fn default() -> Self {
        Self {
            next_raffle: None,
            prize: Amount::nano(1),
            interval: DEFAULT_RAFFLE_INTERVAL,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RaffleResult {
    pub winner: String,
    pub participants: Vec<String>,
    pub prize: Amount,
    pub destination: Account,
}

impl RaffleRunner {
    pub fn reset(&mut self) {
        self.next_raffle = None;
    }

    pub fn set_prize(&mut self, prize: Amount) {
        self.prize = prize;
    }

    pub fn next_raffle(&mut self, now: Timestamp) -> Timestamp {
        match self.next_raffle {
            None => {
                let next = now + self.interval;
                self.next_raffle = Some(next);
                next
            }
            Some(next) => next,
        }
    }

    pub fn raffle_interval(&self) -> Duration {
        self.interval
    }

    pub fn set_raffle_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    pub fn run_raffle_now(&mut self, now: Timestamp) {
        self.next_raffle = Some(now);
    }

    pub fn prize(&self) -> Amount {
        self.prize
    }

    pub fn try_run_raffle(
        &mut self,
        participants: &ParticipantRegistry,
        now: Timestamp,
        random: u32,
    ) -> Option<RaffleResult> {
        let next_raffle = self.next_raffle(now);
        let time_for_raffle = now >= next_raffle;
        if !time_for_raffle {
            return None;
        }

        if let Some(winner) = participants.pick_random(random) {
            self.next_raffle = Some(now + self.raffle_interval());

            Some(RaffleResult {
                winner: winner.name,
                participants: participants.list().drain(..).map(|p| p.name).collect(),
                prize: self.prize(),
                destination: winner.account,
            })
        } else {
            None
        }
    }
}

static DEFAULT_RAFFLE_INTERVAL: Duration = Duration::from_secs(60 * 4);
