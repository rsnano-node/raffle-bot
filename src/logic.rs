use crate::{
    chat::{ChatMessage, LatestChatMessages},
    participants::{Participant, ParticipantRegistry},
    raffle_runner::RaffleRunner,
    upcoming_raffle_announcement::UpcomingRaffleAnnouncement,
};
use rsnano_core::{Account, Amount};
use rsnano_nullable_clock::Timestamp;
use std::time::Duration;

#[derive(Default)]
pub(crate) struct RaffleLogic {
    latest_messages: LatestChatMessages,
    participants: ParticipantRegistry,
    raffle_runner: RaffleRunner,
    upcoming_raffle_announcement: UpcomingRaffleAnnouncement,
}

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

    pub fn raffle_interval(&self) -> Duration {
        self.raffle_runner.raffle_interval()
    }

    pub fn latest_messages(&self) -> impl Iterator<Item = &ChatMessage> {
        self.latest_messages.iter()
    }

    pub fn participants(&self) -> Vec<Participant> {
        self.participants.list()
    }

    pub fn countdown(&mut self, now: Timestamp) -> Duration {
        self.raffle_runner.next_raffle(now) - now
    }

    pub fn tick(&mut self, now: Timestamp, random: u32) -> Vec<Action> {
        let result = self
            .raffle_runner
            .try_run_raffle(&self.participants, now, random);

        if result.raffle_completed {
            self.upcoming_raffle_announcement.raffle_completed();
        }
        let mut actions = result.actions;
        actions.extend(
            self.upcoming_raffle_announcement
                .tick(result.next_raffle, now),
        );
        actions
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
        let actions = app.tick(start + app.raffle_interval(), 0);
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
        let announce_time =
            start + app.raffle_interval() - app.upcoming_raffle_announcement.offset();
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
        let announce_time =
            start + app.raffle_interval() - app.upcoming_raffle_announcement.offset();
        let actions = app.tick(announce_time - Duration::from_secs(1), 0);
        assert_eq!(actions.len(), 0);
        let actions = app.tick(announce_time, 0);
        assert_eq!(actions.len(), 1);
        let actions = app.tick(announce_time + Duration::from_secs(1), 0);
        assert_eq!(actions.len(), 0);
    }
}
