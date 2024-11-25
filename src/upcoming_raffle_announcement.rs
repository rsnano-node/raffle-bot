use crate::logic::Action;
use rsnano_nullable_clock::Timestamp;
use std::time::Duration;

static ANNOUNCEMENT_OFFSET: Duration = Duration::from_secs(10);

#[derive(Default)]
pub(crate) struct UpcomingRaffleAnnouncement {
    pub announcement_made: bool,
}

impl UpcomingRaffleAnnouncement {
    pub fn raffle_completed(&mut self) {
        self.announcement_made = false;
    }

    pub fn offset(&self) -> Duration {
        ANNOUNCEMENT_OFFSET
    }

    pub fn tick(&mut self, next_raffle: Timestamp, now: Timestamp) -> Vec<Action> {
        let mut actions = Vec::new();
        if now >= next_raffle - self.offset() && !self.announcement_made {
            actions.push(Action::Notify(format!(
                "Get ready! The next raffle starts in {} seconds...",
                self.offset().as_secs()
            )));
            self.announcement_made = true;
        }
        actions
    }
}
