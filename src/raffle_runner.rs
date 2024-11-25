use crate::participants::ParticipantRegistry;
use rsnano_core::{Account, Amount};
use rsnano_nullable_clock::Timestamp;
use std::time::Duration;

#[derive(Default)]
pub(crate) struct RaffleRunner {
    next_raffle: Option<Timestamp>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RaffleResult {
    pub winner: String,
    pub participants: Vec<String>,
    pub prize: Amount,
    pub destination: Account,
}

impl RaffleRunner {
    pub fn next_raffle(&mut self, now: Timestamp) -> Timestamp {
        match self.next_raffle {
            None => {
                let next = now + RAFFLE_INTERVAL;
                self.next_raffle = Some(next);
                next
            }
            Some(next) => next,
        }
    }

    pub fn raffle_interval(&self) -> Duration {
        RAFFLE_INTERVAL
    }

    pub fn prize(&self) -> Amount {
        Amount::kxrb(10)
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

static RAFFLE_INTERVAL: Duration = Duration::from_secs(60 * 5);
