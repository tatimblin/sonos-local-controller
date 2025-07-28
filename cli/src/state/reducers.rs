use crate::{topology::{topology_item::TopologyItem, topology_list::TopologyList}, views::ViewType};

use super::store::AppState;

pub enum AppAction {
    SetStatusMessage(String),
    UpdateTopology(TopologyList),
    SetHighlight(TopologyItem),
    SetSelectSpeaker(String),
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
            AppAction::SetHighlight(item) => {
                f.debug_tuple("SetHighlight").field(item).finish()
            }
            AppAction::SetSelectSpeaker(uuid) => {
                f.debug_tuple("SetSelectSpeaker").field(uuid).finish()
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
        AppAction::SetHighlight(item) => {
            log::debug!("SetHighlight action received: {:?}", item.get_type());
            state.highlight = Some(item);
        }
        AppAction::SetSelectSpeaker(uuid) => {
            log::debug!("SetSelectSpeaker action received: {}", uuid);

            let is_currently_locked =
                state.selected_speaker_ip.as_ref().map(|s| s.as_str()) == Some(&uuid);

            if is_currently_locked {
                state.selected_speaker_ip = None;
            } else {
                state.selected_speaker_ip = Some(uuid);
            }
        }
    }
}
