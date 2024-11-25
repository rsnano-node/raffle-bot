use rsnano_core::Account;
use std::collections::HashMap;

#[derive(PartialEq, Eq, Debug, Clone)]
pub(crate) struct Participant {
    pub channel_id: String,
    pub name: String,
    pub account: Account,
}

impl Participant {
    pub fn new_test_instance() -> Self {
        Self {
            channel_id: "abc".to_owned(),
            name: "John Doe".to_owned(),
            account: Account::from(42),
        }
    }

    pub fn new_test_instance_for_channel(channel_id: impl Into<String>) -> Self {
        let channel_id = channel_id.into();
        Self {
            name: format!("name for {}", channel_id),
            channel_id,
            account: Account::from(42),
        }
    }
}

#[derive(Default)]
pub(crate) struct ParticipantRegistry(HashMap<String, Participant>);

impl ParticipantRegistry {
    pub fn add(&mut self, participant: Participant) {
        self.0.insert(participant.channel_id.clone(), participant);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn list(&self) -> Vec<Participant> {
        let mut result: Vec<_> = self.0.values().cloned().collect();
        result.sort_by(|a, b| a.channel_id.cmp(&b.channel_id));
        result
    }

    pub fn pick_random(&self, random_val: u32) -> Option<Participant> {
        if self.len() == 0 {
            return None;
        }

        let mut all_participants = self.list();
        Some(all_participants.remove(random_val as usize % self.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let participants = ParticipantRegistry::default();
        assert_eq!(participants.len(), 0);
    }

    #[test]
    fn register_one() {
        let mut participants = ParticipantRegistry::default();
        let john = Participant::new_test_instance();
        participants.add(john.clone());
        assert_eq!(participants.len(), 1);
        assert_eq!(participants.list(), vec![john]);
    }

    #[test]
    fn register_two() {
        let mut participants = ParticipantRegistry::default();
        let alice = Participant::new_test_instance();
        let bob = Participant {
            channel_id: "xxanother_channel".to_owned(),
            ..Participant::new_test_instance()
        };
        participants.add(alice.clone());
        participants.add(bob.clone());
        assert_eq!(participants.len(), 2);
        let list = participants.list();
        assert_eq!(list.len(), 2);
        assert_eq!(participants.list(), vec![alice, bob]);
    }

    #[test]
    fn replace_old_registration() {
        let mut participants = ParticipantRegistry::default();
        let old = Participant::new_test_instance();
        let new = Participant {
            account: Account::from(99999),
            ..old.clone()
        };
        participants.add(old);
        participants.add(new.clone());
        assert_eq!(participants.len(), 1);
        assert_eq!(participants.list(), vec![new]);
    }

    #[test]
    fn return_participants_ordered_by_channel_id() {
        let mut participants = ParticipantRegistry::default();
        let bob = Participant::new_test_instance_for_channel("a");
        let alice = Participant::new_test_instance_for_channel("b");
        let john = Participant::new_test_instance_for_channel("c");
        participants.add(john.clone());
        participants.add(bob.clone());
        participants.add(alice.clone());
        assert_eq!(participants.list(), vec![bob, alice, john])
    }

    #[test]
    fn pick_random_one_entry() {
        let mut participants = ParticipantRegistry::default();
        let john = Participant::new_test_instance_for_channel("a");

        participants.add(john.clone());

        assert_eq!(participants.pick_random(1).unwrap(), john);
        assert_eq!(participants.pick_random(2).unwrap(), john);
    }

    #[test]
    fn pick_random() {
        let mut participants = ParticipantRegistry::default();
        let bob = Participant::new_test_instance_for_channel("a");
        let alice = Participant::new_test_instance_for_channel("b");
        let john = Participant::new_test_instance_for_channel("c");

        participants.add(bob.clone());
        participants.add(alice.clone());
        participants.add(john.clone());

        assert_eq!(participants.pick_random(0).unwrap(), bob);
        assert_eq!(participants.pick_random(1).unwrap(), alice);
        assert_eq!(participants.pick_random(2).unwrap(), john);
        assert_eq!(participants.pick_random(3).unwrap(), bob);
    }
}
