use std::collections::HashMap;

use sonos::SpeakerInfo;

use crate::{
    topology::{topology_item::TopologyItem, topology_list::TopologyList},
    views::ViewType,
};

use super::store::AppState;

pub enum AppAction {
    SetStatusMessage(String),
    UpdateTopology(TopologyList),
    SetHighlight(TopologyItem),
    SetSelectSpeaker(String),
    SetControlView,
    HydrateSpeakerTopology(SpeakerInfo),
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
            AppAction::SetHighlight(item) => f.debug_tuple("SetHighlight").field(item).finish(),
            AppAction::SetSelectSpeaker(uuid) => {
                f.debug_tuple("SetSelectSpeaker").field(uuid).finish()
            }
            AppAction::SetControlView => f.debug_tuple("SetControlView").finish(),
            AppAction::HydrateSpeakerTopology(speaker_info) => f
                .debug_tuple("HydrateSpeakerTopology")
                .field(&speaker_info.uuid)
                .finish(),
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
            let topology_map = create_uuid_to_index_map(&topology);
            state.topology = Some(topology);
            state.topology_ref = Some(topology_map);
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
        AppAction::SetControlView => {
            state.view = ViewType::Control;
        }
        AppAction::HydrateSpeakerTopology(speaker_info) => {
            log::debug!(
                "HydrateSpeakerTopology action received for UUID: {}",
                speaker_info.uuid
            );

            if let Some(ref mut topology) = state.topology {
                if !topology.items.is_empty() {
                    if let Some(ref topology_ref) = state.topology_ref {
                        let normalized_uuid = normalize_uuid(&speaker_info.uuid);
                        if let Some(&index) = topology_ref.get(&normalized_uuid) {
                            if let Some(item) = topology.items.get_mut(index) {
                                match item {
                                    TopologyItem::Speaker { model, .. } => {
                                        *model = Some(speaker_info.model.clone());
                                    }
                                    TopologyItem::Group { name, .. } => {
                                        *name = format!(
                                            "{} ({})",
                                            speaker_info.name, speaker_info.model
                                        );
                                    }
                                    TopologyItem::Satellite { .. } => {
                                        log::debug!("Skipping Satellite item (no name field)");
                                    }
                                }
                            }
                        } else {
                            log::debug!("UUID {} not found in topology_ref", normalized_uuid);
                        }
                    }
                } else {
                    log::debug!("Topology is empty");
                }
            } else {
                log::debug!("No topology available");
            }
        }
    }
}

fn create_uuid_to_index_map(list: &TopologyList) -> HashMap<String, usize> {
    list.items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.get_uuid().to_string(), index))
        .collect()
}

fn normalize_uuid(uuid: &str) -> String {
    // Remove "uuid:" prefix if present
    let without_prefix = if uuid.starts_with("uuid:") {
        &uuid[5..]
    } else {
        uuid
    };

    // Remove "::urn:schemas-upnp-org:device:ZonePlayer:1" suffix if present
    if let Some(pos) = without_prefix.find("::") {
        without_prefix[..pos].to_string()
    } else {
        without_prefix.to_string()
    }
}
