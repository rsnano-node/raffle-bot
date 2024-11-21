use crate::{
    chat::ChatMessage,
    latest_chat_messages::LatestChatMessages,
    registered_viewers::{RegisteredViewer, ViewerRegistry},
};
use rsnano_core::{Account, Amount};
use rsnano_nullable_clock::Timestamp;
use std::time::Duration;

#[derive(Default)]
pub(crate) struct RaffleLogic {
    latest_messages: LatestChatMessages,
    viewer_registry: ViewerRegistry,
    next_raffle: Option<Timestamp>,
    announcement_made: bool,
}

static RAFFLE_INTERVAL: Duration = Duration::from_secs(60 * 5);

impl RaffleLogic {
    pub fn handle_chat_message(&mut self, message: ChatMessage) {
        for word in message.message.split_whitespace() {
            if let Ok(account) = Account::decode_account(word) {
                self.viewer_registry.add(RegisteredViewer {
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

    pub fn registered_viewers(&self) -> Vec<RegisteredViewer> {
        self.viewer_registry.list()
    }

    pub fn countdown(&self, now: Timestamp) -> Duration {
        match self.next_raffle {
            None => RAFFLE_INTERVAL,
            Some(next) => next - now,
        }
    }

    pub fn tick(&mut self, now: Timestamp, random: u32) -> Vec<OutputAction> {
        let mut actions = Vec::new();
        match self.next_raffle {
            None => {
                self.next_raffle = Some(now + RAFFLE_INTERVAL);
            }
            Some(next) => {
                if now >= next {
                    self.next_raffle = Some(now + RAFFLE_INTERVAL);
                    self.announcement_made = false;
                    if let Some(winner) = self.viewer_registry.pick_random(random) {
                        let amount = Amount::nano(1);

                        actions.push(OutputAction::Notify(format!(
                            "Congratulations {}! You've just won Ӿ {}",
                            winner.name,
                            amount.format_balance(1)
                        )));

                        actions.push(OutputAction::SendToWinner(Winner {
                            name: winner.name,
                            amount,
                            account: winner.account,
                        }));
                    }
                } else if now >= next - Duration::from_secs(30) && !self.announcement_made {
                    actions.push(OutputAction::Notify(
                        "Get ready! The next raffle starts in 30 seconds...".to_owned(),
                    ));
                    self.announcement_made = true;
                }
            }
        }
        actions
    }
}

#[derive(PartialEq, Eq, Debug)]
pub(crate) enum OutputAction {
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
        assert_eq!(app.registered_viewers().len(), 0);
    }

    #[test]
    fn receive_chat_message() {
        let mut app = RaffleLogic::default();
        let message = ChatMessage::new_test_instance();
        app.handle_chat_message(message);
        assert_eq!(app.latest_messages().count(), 1);
        assert_eq!(app.registered_viewers().len(), 0);
    }

    #[test]
    fn register_viewer() {
        let mut app = RaffleLogic::default();
        let message = ChatMessage {
            message: "My address is nano_37391u1nrr1j7tdn8w9zathoio5suz9bar18jksqheeiy4obwz3pkgp9aqz6 :-)".to_owned(), 
            ..ChatMessage::new_test_instance()
        };

        app.handle_chat_message(message.clone());

        let registered = app.registered_viewers();
        assert_eq!(registered.len(), 1);
        assert_eq!(
            registered[0],
            RegisteredViewer {
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
        assert_eq!(actions.len(), 2);
        assert_eq!(
            actions[1],
            OutputAction::SendToWinner(Winner {
                name: msg.author_name.unwrap(),
                amount: Amount::nano(1),
                account
            })
        )
    }
}
