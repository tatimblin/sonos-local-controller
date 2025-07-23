use crate::{topology::topology_list::TopologyList, views::ViewType};

use super::store::AppState;

pub enum AppAction {
    SetStatusMessage(String),
    UpdateTopology(TopologyList),
    SetActiveSpeaker(String),
    ToggleSpeakerLock(String),
}

impl std::fmt::Debug for AppAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppAction::SetStatusMessage(message) => {
                f.debug_tuple("SetStatusMessage").field(message).finish()
            }
            AppAction::UpdateTopology(topology) => {
                f.debug_tuple("UpdateTopology").field(topology).finish()
            }
            AppAction::SetActiveSpeaker(uuid) => {
                f.debug_tuple("SetActiveSpeaker").field(uuid).finish()
            }
            AppAction::ToggleSpeakerLock(uuid) => {
                f.debug_tuple("ToggleSpeakerLock").field(uuid).finish()
            }
        }
    }
}

pub fn app_reducer(state: &mut AppState, action: AppAction) {
    match action {
        AppAction::SetStatusMessage(message) => {
            state.status_message = message;
        }
        AppAction::UpdateTopology(topology) => {
            log::debug!("SetTopology action received, switching to Control view");
            state.topology = Some(topology);
            state.view = ViewType::Control;
        }
        AppAction::SetActiveSpeaker(uuid) => {
            log::debug!("SetActiveSpeaker action received: {}", uuid);
            state.active_speaker_uuid = Some(uuid);
        }
        AppAction::ToggleSpeakerLock(uuid) => {
            log::debug!("ToggleSpeakerLock action received: {}", uuid);

            // Toggle the lock state - if currently locked to this speaker, unlock it
            // If locked to a different speaker or not locked, lock to this speaker
            let is_currently_locked =
                state.locked_speaker_uuid.as_ref().map(|s| s.as_str()) == Some(&uuid);

            if is_currently_locked {
                state.locked_speaker_uuid = None;
            } else {
                // Ensure single selection constraint - automatically unlock any previously locked speaker
                state.locked_speaker_uuid = Some(uuid);
            }
        }
    }
}
