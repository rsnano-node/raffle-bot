use crate::{
    chat::{ChatMessage, LatestChatMessages},
    participants::{Participant, ParticipantRegistry},
    upcoming_raffle_announcement::UpcomingRaffleAnnouncement,
};
use rsnano_core::{Account, Amount};
use rsnano_nullable_clock::Timestamp;
use std::time::Duration;

#[derive(Default)]
pub(crate) struct RaffleLogic {
    latest_messages: LatestChatMessages,
    participants: ParticipantRegistry,
    upcoming_raffle_announcement: UpcomingRaffleAnnouncement,
    next_raffle: Option<Timestamp>,
}

static RAFFLE_INTERVAL: Duration = Duration::from_secs(60 * 5);

impl RaffleLogic {
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

    pub fn latest_messages(&self) -> impl Iterator<Item = &ChatMessage> {
        self.latest_messages.iter()
    }

    pub fn participants(&self) -> Vec<Participant> {
        self.participants.list()
    }

    pub fn countdown(&mut self, now: Timestamp) -> Duration {
        self.next_raffle(now) - now
    }

    pub fn tick(&mut self, now: Timestamp, random: u32) -> Vec<Action> {
        let (mut actions, next_raffle) = self.try_run_raffle(now, random);
        actions.extend(self.upcoming_raffle_announcement.tick(next_raffle, now));
        actions
    }

    fn next_raffle(&mut self, now: Timestamp) -> Timestamp {
        match self.next_raffle {
            None => {
                let next = now + RAFFLE_INTERVAL;
                self.next_raffle = Some(next);
                next
            }
            Some(next) => next,
        }
    }

    fn try_run_raffle(&mut self, now: Timestamp, random: u32) -> (Vec<Action>, Timestamp) {
        let mut next_raffle = self.next_raffle(now);
        let mut actions = Vec::new();
        let time_for_raffle = now >= next_raffle;
        if time_for_raffle {
            if let Some(winner) = self.participants.pick_random(random) {
                actions.extend(self.reward_winner(winner));
            }
            self.upcoming_raffle_announcement.raffle_completed();
            next_raffle = now + RAFFLE_INTERVAL;
            self.next_raffle = Some(next_raffle);
        }
        (actions, next_raffle)
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

#[derive(PartialEq, Eq, Debug)]
pub(crate) enum Action {
    SendToWinner(Winner),
    Notify(String),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Winner {
    pub name: String,
    pub amount: Amount,
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
        let mut app = RaffleLogic::default();
        let actions = app.tick(Timestamp::new_test_instance(), 1);
        assert!(actions.is_empty());
    }

    #[test]
    fn pick_single_winner() {
        let mut app = RaffleLogic::default();
        let start = Timestamp::new_test_instance();
        app.tick(start, 0);
        let account = Account::from(42);
        let msg = ChatMessage::new_test_instance_for_account(account);
        app.handle_chat_message(msg.clone());
        let actions = app.tick(start + RAFFLE_INTERVAL, 0);
        assert!(actions.len() > 0);
        assert_eq!(
            actions.last().unwrap(),
            &Action::SendToWinner(Winner {
                name: msg.author_name.unwrap(),
                amount: Amount::nano(1),
                account
            })
        )
    }

    #[test]
    fn announce_next_raffle() {
        let mut app = RaffleLogic::default();
        let start = Timestamp::new_test_instance();
        app.tick(start, 0);
        let announce_time = start + RAFFLE_INTERVAL - app.upcoming_raffle_announcement.offset();
        assert!(app
            .tick(announce_time - Duration::from_secs(1), 0)
            .is_empty());
        let actions = app.tick(announce_time, 0);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            Action::Notify("Get ready! The next raffle starts in 10 seconds...".to_owned())
        );
    }

    #[test]
    fn announce_only_once() {
        let mut app = RaffleLogic::default();
        let start = Timestamp::new_test_instance();
        app.tick(start, 0);
        let announce_time = start + RAFFLE_INTERVAL - app.upcoming_raffle_announcement.offset();
        let actions = app.tick(announce_time - Duration::from_secs(1), 0);
        assert_eq!(actions.len(), 0);
        let actions = app.tick(announce_time, 0);
        assert_eq!(actions.len(), 1);
        let actions = app.tick(announce_time + Duration::from_secs(1), 0);
        assert_eq!(actions.len(), 0);
    }
}
