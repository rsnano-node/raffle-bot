use rsnano_core::Amount;
use rsnano_nullable_clock::Timestamp;
use std::time::Duration;

use crate::{
    logic::{Action, Winner},
    participants::{Participant, ParticipantRegistry},
};

#[derive(Default)]
pub(crate) struct RaffleRunner {
    next_raffle: Option<Timestamp>,
}

pub(crate) struct RaffleResult {
    pub actions: Vec<Action>,
    pub next_raffle: Timestamp,
    pub raffle_completed: bool,
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

    pub fn try_run_raffle(
        &mut self,
        participants: &ParticipantRegistry,
        now: Timestamp,
        random: u32,
    ) -> RaffleResult {
        let mut result = RaffleResult {
            next_raffle: self.next_raffle(now),
            actions: Vec::new(),
            raffle_completed: false,
        };
        let time_for_raffle = now >= result.next_raffle;
        if time_for_raffle {
            if let Some(winner) = participants.pick_random(random) {
                result.actions.extend(self.reward_winner(winner));
            }
            result.next_raffle = now + self.raffle_interval();
            self.next_raffle = Some(result.next_raffle);
            result.raffle_completed = true;
        }

        result
    }

    fn reward_winner(&self, winner: Participant) -> Vec<Action> {
        let amount = Amount::nano(1);

        let notify = Action::Notify(format!(
            "Congratulations {}! You've just won Ӿ {}",
            winner.name,
            amount.format_balance(1)
        ));

        let send_prize = Action::SendToWinner(Winner {
            name: winner.name,
            amount,
            account: winner.account,
        });

        vec![notify, send_prize]
    }
}

static RAFFLE_INTERVAL: Duration = Duration::from_secs(60 * 5);
