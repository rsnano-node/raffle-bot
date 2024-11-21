use rsnano_core::Account;
use std::collections::HashMap;

#[derive(PartialEq, Eq, Debug, Clone)]
pub(crate) struct RegisteredViewer {
    pub channel_id: String,
    pub name: String,
    pub account: Account,
}

impl RegisteredViewer {
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
pub(crate) struct ViewerRegistry(HashMap<String, RegisteredViewer>);

impl ViewerRegistry {
    pub fn add(&mut self, viewer: RegisteredViewer) {
        self.0.insert(viewer.channel_id.clone(), viewer);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn list(&self) -> Vec<RegisteredViewer> {
        let mut result: Vec<_> = self.0.values().cloned().collect();
        result.sort_by(|a, b| a.channel_id.cmp(&b.channel_id));
        result
    }

    pub fn pick_random(&self, random_val: u32) -> Option<RegisteredViewer> {
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
    use rsnano_nullable_random::NullableRng;

    #[test]
    fn empty() {
        let registered_viewers = ViewerRegistry::default();
        assert_eq!(registered_viewers.len(), 0);
    }

    #[test]
    fn register_one_viewer() {
        let mut registered_viewers = ViewerRegistry::default();
        let viewer = RegisteredViewer::new_test_instance();
        registered_viewers.add(viewer.clone());
        assert_eq!(registered_viewers.len(), 1);
        assert_eq!(registered_viewers.list(), vec![viewer]);
    }

    #[test]
    fn register_two_viewers() {
        let mut registered_viewers = ViewerRegistry::default();
        let viewer1 = RegisteredViewer::new_test_instance();
        let viewer2 = RegisteredViewer {
            channel_id: "xxanother_channel".to_owned(),
            ..RegisteredViewer::new_test_instance()
        };
        registered_viewers.add(viewer1.clone());
        registered_viewers.add(viewer2.clone());
        assert_eq!(registered_viewers.len(), 2);
        let list = registered_viewers.list();
        assert_eq!(list.len(), 2);
        assert_eq!(registered_viewers.list(), vec![viewer1, viewer2]);
    }

    #[test]
    fn replace_old_registration() {
        let mut registered_viewers = ViewerRegistry::default();
        let viewer_old = RegisteredViewer::new_test_instance();
        let viewer_new = RegisteredViewer {
            account: Account::from(99999),
            ..viewer_old.clone()
        };
        registered_viewers.add(viewer_old);
        registered_viewers.add(viewer_new.clone());
        assert_eq!(registered_viewers.len(), 1);
        assert_eq!(registered_viewers.list(), vec![viewer_new]);
    }

    #[test]
    fn return_users_ordered_by_channel_id() {
        let mut registered_viewers = ViewerRegistry::default();
        let viewer_a = RegisteredViewer::new_test_instance_for_channel("a");
        let viewer_b = RegisteredViewer::new_test_instance_for_channel("b");
        let viewer_c = RegisteredViewer::new_test_instance_for_channel("c");
        registered_viewers.add(viewer_c.clone());
        registered_viewers.add(viewer_a.clone());
        registered_viewers.add(viewer_b.clone());
        assert_eq!(
            registered_viewers.list(),
            vec![viewer_a, viewer_b, viewer_c]
        )
    }

    #[test]
    fn pick_random_one_entry() {
        let mut registered_viewers = ViewerRegistry::default();
        let viewer = RegisteredViewer::new_test_instance_for_channel("a");

        registered_viewers.add(viewer.clone());

        assert_eq!(registered_viewers.pick_random(1).unwrap(), viewer);
        assert_eq!(registered_viewers.pick_random(2).unwrap(), viewer);
    }

    #[test]
    fn pick_random() {
        let mut registered_viewers = ViewerRegistry::default();
        let viewer_a = RegisteredViewer::new_test_instance_for_channel("a");
        let viewer_b = RegisteredViewer::new_test_instance_for_channel("b");
        let viewer_c = RegisteredViewer::new_test_instance_for_channel("c");

        registered_viewers.add(viewer_a.clone());
        registered_viewers.add(viewer_b.clone());
        registered_viewers.add(viewer_c.clone());

        assert_eq!(registered_viewers.pick_random(0).unwrap(), viewer_a);
        assert_eq!(registered_viewers.pick_random(1).unwrap(), viewer_b);
        assert_eq!(registered_viewers.pick_random(2).unwrap(), viewer_c);
        assert_eq!(registered_viewers.pick_random(3).unwrap(), viewer_a);
    }
}
