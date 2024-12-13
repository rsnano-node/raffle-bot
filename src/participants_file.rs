use crate::participants::Participant;
use log::{info, warn};
use std::collections::HashSet;

#[derive(Default)]
pub(crate) struct ParticipantsFile {
    last_saved: HashSet<Participant>,
}

impl ParticipantsFile {
    pub(crate) fn load(&mut self) -> Vec<Participant> {
        let participants = load_participants();
        for p in &participants {
            self.last_saved.insert(p.clone());
        }
        participants
    }

    pub(crate) fn update(&mut self, participants: Vec<Participant>) {
        let participants: HashSet<Participant> = HashSet::from_iter(participants);
        if self
            .last_saved
            .symmetric_difference(&participants)
            .next()
            .is_none()
        {
            // nothing changed
            return;
        }

        self.last_saved = participants;

        match std::fs::write(
            FILE_PATH,
            serde_json::to_string_pretty(&self.last_saved).unwrap(),
        ) {
            Ok(_) => {
                info!("Participants file written")
            }
            Err(e) => warn!("Could not save participants file: {:?}", e),
        }
    }
}

fn load_participants() -> Vec<Participant> {
    let Ok(json) = std::fs::read_to_string(FILE_PATH) else {
        warn!("Could not read participants.json");
        return Vec::new();
    };

    match serde_json::from_str(&json) {
        Ok(participants) => participants,
        Err(e) => {
            warn!("Could not deserialize participants file: {:?}", e);
            Vec::new()
        }
    }
}

const FILE_PATH: &str = "participants.json";
