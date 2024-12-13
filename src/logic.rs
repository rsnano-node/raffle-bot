use crate::{
    chat_messages::{ChatMessage, LatestChatMessages},
    participants::{Participant, ParticipantRegistry},
    raffle_runner::{RaffleResult, RaffleRunner},
};
use rsnano_core::{Account, Amount};
use rsnano_nullable_clock::Timestamp;
use std::time::Duration;

#[derive(Default)]
pub(crate) struct RaffleLogic {
    latest_messages: LatestChatMessages,
    participants: ParticipantRegistry,
    raffle_runner: RaffleRunner,
    current_win: Option<RaffleResult>,
    spin_finished: bool,
    running: bool,
    last_ping: Option<Timestamp>,
    winners: Vec<String>,
}

impl RaffleLogic {
    pub fn running(&self) -> bool {
        self.running
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
        self.raffle_runner.reset();
    }

    pub fn handle_chat_message(&mut self, message: ChatMessage) {
        for word in message.message.split_whitespace() {
            if let Ok(account) = Account::decode_account(word) {
                self.participants.add(Participant {
                    channel_id: message.author_channel_id.clone(),
                    name: message
                        .author_name
                        .clone()
                        .unwrap_or_else(|| "no name".to_string()),
                    account,
                });
            }
        }
        self.latest_messages.add(message);
    }

    pub fn set_prize(&mut self, prize: Amount) {
        self.raffle_runner.set_prize(prize);
    }

    pub fn raffle_interval(&self) -> Duration {
        self.raffle_runner.raffle_interval()
    }

    pub fn set_raffle_interval(&mut self, interval: Duration) {
        self.raffle_runner.set_raffle_interval(interval);
    }

    pub fn run_raffle_now(&mut self, now: Timestamp) {
        self.raffle_runner.run_raffle_now(now);
    }

    pub fn prize(&self) -> Amount {
        self.raffle_runner.prize()
    }

    pub fn ping(&mut self, now: Timestamp) {
        self.last_ping = Some(now);
    }

    pub fn spinner_connected(&self, now: Timestamp) -> bool {
        self.last_ping
            .map(|p| now - p < Duration::from_secs(3))
            .unwrap_or_default()
    }

    pub fn current_win(&self) -> Option<&RaffleResult> {
        if self.spin_finished {
            None
        } else {
            self.current_win.as_ref()
        }
    }

    pub fn latest_messages(&self) -> impl Iterator<Item = &ChatMessage> {
        self.latest_messages.iter()
    }

    pub fn participants(&self) -> Vec<Participant> {
        self.participants.list()
    }

    pub fn winners(&self) -> &Vec<String> {
        &self.winners
    }

    pub fn countdown(&mut self, now: Timestamp) -> Duration {
        if !self.running {
            return Duration::ZERO;
        }

        let next = self.raffle_runner.next_raffle(now);
        if now >= next {
            Duration::ZERO
        } else {
            next - now
        }
    }

    pub fn tick(&mut self, now: Timestamp, random: u32) -> Vec<Action> {
        if !self.running {
            return Vec::new();
        }

        let result = self
            .raffle_runner
            .try_run_raffle(&self.participants, now, random);

        if result.is_some() {
            self.spin_finished = false;
            self.current_win = result;
        }

        let mut actions = Vec::new();

        if self.spin_finished {
            if let Some(win) = self.current_win.take() {
                self.winners.push(win.winner.clone());
                actions.extend(self.reward_winner(win));
                self.spin_finished = false;
            }
        }

        actions
    }

    pub fn spin_finished(&mut self) {
        self.spin_finished = true;
    }

    fn reward_winner(&self, result: RaffleResult) -> Vec<Action> {
        let notify = Action::Notify(format!(
            "Congratulations {}! You've just won Ӿ {}",
            result.winner,
            result.prize.format_balance(2)
        ));

        let send_prize = Action::SendToWinner(Winner {
            name: result.winner,
            prize: result.prize,
            account: result.destination,
        });

        vec![notify, send_prize]
    }
}

#[derive(PartialEq, Eq, Debug)]
pub(crate) enum Action {
    SendToWinner(Winner),
    Notify(String),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Winner {
    pub name: String,
    pub prize: Amount,
    pub account: Account,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let app = RaffleLogic::default();
        assert_eq!(app.latest_messages().count(), 0);
        assert_eq!(app.participants().len(), 0);
    }

    #[test]
    fn receive_chat_message() {
        let mut app = RaffleLogic::default();
        let message = ChatMessage::new_test_instance();
        app.handle_chat_message(message);
        assert_eq!(app.latest_messages().count(), 1);
        assert_eq!(app.participants().len(), 0);
    }

    #[test]
    fn register_viewer() {
        let mut app = RaffleLogic::default();
        let message = ChatMessage {
            message: "My address is nano_37391u1nrr1j7tdn8w9zathoio5suz9bar18jksqheeiy4obwz3pkgp9aqz6 :-)".to_owned(), 
            ..ChatMessage::new_test_instance()
        };

        app.handle_chat_message(message.clone());

        let registered = app.participants();
        assert_eq!(registered.len(), 1);
        assert_eq!(
            registered[0],
            Participant {
                channel_id: message.author_channel_id,
                name: message.author_name.unwrap(),
                account: Account::decode_account(
                    "nano_37391u1nrr1j7tdn8w9zathoio5suz9bar18jksqheeiy4obwz3pkgp9aqz6"
                )
                .unwrap()
            }
        );
    }

    #[test]
    fn tick_empty() {
        let mut logic = RaffleLogic::default();
        logic.start();
        let actions = logic.tick(Timestamp::new_test_instance(), 1);
        assert!(actions.is_empty());
    }

    #[test]
    fn pick_single_winner() {
        let mut logic = RaffleLogic::default();
        logic.start();
        let start = Timestamp::new_test_instance();
        logic.tick(start, 0);
        let account = Account::from(42);
        let msg = ChatMessage::new_test_instance_for_account(account);
        let viewer = msg.author_name.as_ref().unwrap().clone();
        logic.handle_chat_message(msg.clone());
        let actions = logic.tick(start + logic.raffle_interval(), 0);
        assert_eq!(actions.len(), 0);
        assert_eq!(
            logic.current_win(),
            Some(&RaffleResult {
                winner: viewer.clone(),
                participants: vec![viewer.clone()],
                prize: logic.prize(),
                destination: account
            })
        );
        logic.spin_finished();

        let actions = logic.tick(start + logic.raffle_interval(), 0);
        assert!(actions.len() > 1);
        assert_eq!(
            actions.last().unwrap(),
            &Action::SendToWinner(Winner {
                name: viewer,
                prize: logic.prize(),
                account
            })
        );
        assert!(logic.current_win().is_none());
    }
}
