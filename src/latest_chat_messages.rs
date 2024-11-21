use crate::chat::ChatMessage;
use std::collections::VecDeque;

#[derive(Default)]
pub(crate) struct LatestChatMessages(VecDeque<ChatMessage>);

impl LatestChatMessages {
    const MAX_MESSAGES: usize = 30;
    pub fn add(&mut self, message: ChatMessage) {
        self.0.push_back(message);
        if self.0.len() > Self::MAX_MESSAGES {
            self.0.pop_front();
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ChatMessage> {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn empty() {
        let msgs = LatestChatMessages::default();
        assert_eq!(msgs.iter().count(), 0);
    }

    #[test]
    pub fn record_message() {
        let mut msgs = LatestChatMessages::default();
        let message = ChatMessage::new_test_instance();
        msgs.add(message.clone());
        assert_eq!(msgs.iter().collect::<Vec<_>>(), vec![&message]);
    }

    #[test]
    pub fn limit_to_last_30() {
        let mut msgs = LatestChatMessages::default();
        for i in 0..40 {
            let message = ChatMessage {
                message: format!("message number {}", i),
                ..ChatMessage::new_test_instance()
            };
            msgs.add(message);
        }
        let collected: Vec<_> = msgs.iter().collect();
        assert_eq!(collected.len(), 30);
        assert_eq!(collected[0].message, "message number 10");
        assert_eq!(collected[29].message, "message number 39");
    }
}
