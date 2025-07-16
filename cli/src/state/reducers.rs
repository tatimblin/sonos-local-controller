use sonos::{Speaker, System};
use std::sync::Arc;

use crate::{topology::topology_list::TopologyList, views::ViewType};

use super::store::AppState;

pub enum AppAction {
    AddSpeaker(Speaker),
    AdjustVolume(i8),
    SetStatusMessage(String),
    SetTopology(TopologyList),
    SetSystem(Arc<System>),
    SetActiveSpeaker(String),
    LockSpeaker(String),
    UnlockSpeaker,
    ClearActiveSelection,
    ClearSelection,
}

impl std::fmt::Debug for AppAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppAction::AddSpeaker(speaker) => f.debug_tuple("AddSpeaker").field(speaker).finish(),
            AppAction::AdjustVolume(adjustment) => {
                f.debug_tuple("AdjustVolume").field(adjustment).finish()
            }
            AppAction::SetStatusMessage(message) => {
                f.debug_tuple("SetStatusMessage").field(message).finish()
            }
            AppAction::SetTopology(topology) => {
                f.debug_tuple("SetTopology").field(topology).finish()
            }
            AppAction::SetSystem(_) => f.debug_tuple("SetSystem").field(&"Arc<System>").finish(),
            AppAction::SetActiveSpeaker(uuid) => {
                f.debug_tuple("SetActiveSpeaker").field(uuid).finish()
            }
            AppAction::LockSpeaker(uuid) => f.debug_tuple("LockSpeaker").field(uuid).finish(),
            AppAction::UnlockSpeaker => f.debug_tuple("UnlockSpeaker").finish(),
            AppAction::ClearActiveSelection => f.debug_tuple("ClearActiveSelection").finish(),
            AppAction::ClearSelection => f.debug_tuple("ClearSelection").finish(),
        }
    }
}

pub fn app_reducer(state: &mut AppState, action: AppAction) {
    match action {
        AppAction::AddSpeaker(_speaker) => {}
        AppAction::AdjustVolume(_adjustment) => {}
        AppAction::SetStatusMessage(message) => {
            state.status_message = message;
        }
        AppAction::SetTopology(topology) => {
            log::debug!("SetTopology action received, switching to Control view");

            // TODO (ttimblin): check if should clear selected

            state.topology = Some(topology);
            state.view = ViewType::Control;
        }
        AppAction::SetSystem(system) => {
            log::debug!("SetSystem action received");
            state.system = Some(system);
        }
        AppAction::SetActiveSpeaker(uuid) => {
            log::debug!("SetActiveSpeaker action received: {}", uuid);
            state.active_speaker_uuid = Some(uuid);
        }
        AppAction::LockSpeaker(uuid) => {
            log::debug!("LockSpeaker action received: {}", uuid);
            state.locked_speaker_uuid = Some(uuid);
        }
        AppAction::UnlockSpeaker => {
            log::debug!("UnlockSpeaker action received");
            state.locked_speaker_uuid = None;
        }
        AppAction::ClearActiveSelection => {
            log::debug!("ClearActiveSelection action received");
            state.active_speaker_uuid = None;
            state.locked_speaker_uuid = None;
        }
        AppAction::ClearSelection => {
            log::debug!("ClearSelection action received");
            state.active_speaker_uuid = None;
            state.locked_speaker_uuid = None;
        }
    }
}
