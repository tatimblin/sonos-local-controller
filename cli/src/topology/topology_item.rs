use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::ListItem,
};
use sonos::{PlayState, Satellite, SpeakerController, ZoneGroup, ZoneGroupMember};

#[derive(Debug, Clone, PartialEq)]
pub enum TopologyItem {
    Group {
        ip: String,
        name: String,
        uuid: String,
        is_last: bool,
        play_state: PlayState,
    },
    Speaker {
        ip: String,
        uuid: String,
        name: String,
        model: Option<String>,
        is_last: bool,
    },
    Satellite {
        uuid: String,
        is_last: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TopologyType {
    Group,
    Speaker,
    Satellite,
}

impl TopologyItem {
    pub fn from_group(group: &ZoneGroup) -> Self {
        let ip = group.get_coordinator().get_ip();
        let controller = SpeakerController::new();
        let play_state = controller.get_play_state(&ip).unwrap_or(PlayState::Stopped);

        TopologyItem::Group {
            ip,
            uuid: group.id.to_string(),
            name: group.get_name().to_string(),
            is_last: false,
            play_state,
        }
    }

    pub fn from_speaker(speaker: &ZoneGroupMember) -> Self {
        TopologyItem::Speaker {
            ip: speaker.get_ip(),
            name: speaker.zone_name.to_string(),
            uuid: speaker.uuid.to_string(),
            model: None,
            is_last: false,
        }
    }

    pub fn from_satellite(satellite: &Satellite) -> Self {
        TopologyItem::Satellite {
            uuid: satellite.uuid.to_string(),
            is_last: false,
        }
    }

    pub fn get_type(&self) -> TopologyType {
        match self {
            TopologyItem::Group { .. } => TopologyType::Group,
            TopologyItem::Speaker { .. } => TopologyType::Speaker,
            TopologyItem::Satellite { .. } => TopologyType::Satellite,
        }
    }

    pub fn get_uuid(&self) -> &str {
        match self {
            TopologyItem::Group { uuid, .. }
            | TopologyItem::Speaker { uuid, .. }
            | TopologyItem::Satellite { uuid, .. } => uuid,
        }
    }

    pub fn set_is_last(&mut self, is_last: bool) {
        match self {
            TopologyItem::Group {
                is_last: ref mut last,
                ..
            }
            | TopologyItem::Speaker {
                is_last: ref mut last,
                ..
            }
            | TopologyItem::Satellite {
                is_last: ref mut last,
                ..
            } => {
                *last = is_last;
            }
        }
    }

    /// Converts this TopologyItem to a ListItem for use in SelectableList
    pub fn to_list_item(&self) -> ListItem<'static> {
        match self {
            TopologyItem::Group { .. } => self.group_to_list_item(),
            TopologyItem::Speaker { .. } => self.speaker_to_list_item(),
            TopologyItem::Satellite { .. } => self.satellite_to_list_item(),
        }
    }

    /// Converts a Group variant to a ListItem
    fn group_to_list_item(&self) -> ListItem<'static> {
        if let TopologyItem::Group {
            name, play_state, ..
        } = self
        {
            let line = Line::from(vec![
                Span::raw(get_play_state_icon(play_state)),
                Span::raw(name.clone()),
            ]);
            ListItem::new(line)
        } else {
            panic!("group_to_list_item called on non-Group variant")
        }
    }

    /// Converts a Speaker variant to a ListItem
    fn speaker_to_list_item(&self) -> ListItem<'static> {
        if let TopologyItem::Speaker {
            name,
            model,
            is_last,
            ..
        } = self
        {
            let prefix = if *is_last { "└─ " } else { "├─ " };
            let mut spans = vec![Span::raw("  "), Span::raw(prefix), Span::raw(name.clone())];

            if let Some(model_name) = model {
                spans.push(Span::styled(" • ", Style::default().fg(Color::Gray)));
                spans.push(Span::styled(
                    model_name.clone(),
                    Style::default().fg(Color::Gray),
                ));
            }

            let line = Line::from(spans);
            ListItem::new(line)
        } else {
            panic!("speaker_to_list_item called on non-Speaker variant")
        }
    }

    /// Converts a Satellite variant to a ListItem
    fn satellite_to_list_item(&self) -> ListItem<'static> {
        if let TopologyItem::Satellite { uuid, .. } = self {
            let line = Line::from(vec![
                Span::raw("  "),
                Span::styled("Satellite: ", Style::default().fg(Color::Yellow)),
                Span::raw(uuid.clone()),
            ]);
            ListItem::new(line)
        } else {
            panic!("satellite_to_list_item called on non-Satellite variant")
        }
    }
}

fn get_play_state_icon(state: &PlayState) -> String {
    let char = match state {
        PlayState::Playing => "▶ ",
        PlayState::Transitioning => "▶ ",
        PlayState::Paused => "⏸ ",
        PlayState::Stopped => "◼ ",
    };
    char.to_string()
}

#[cfg(test)]
mod tests {
    use sonos::{Topology, VanishedDevice, VanishedDevices};

    use super::*;

    fn create_test_satellite() -> Satellite {
        Satellite {
            uuid: "RINCON_SAT123".to_string(),
            location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
            zone_name: "Satellite Speaker".to_string(),
            software_version: "56.0-76060".to_string(),
        }
    }

    fn create_test_zone_group_member() -> ZoneGroupMember {
        ZoneGroupMember {
            uuid: "RINCON_123456".to_string(),
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            zone_name: "Living Room".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![create_test_satellite()],
        }
    }

    fn create_test_zone_group() -> ZoneGroup {
        ZoneGroup {
            coordinator: "RINCON_123456".to_string(),
            id: "RINCON_123456:1234567890".to_string(),
            members: vec![create_test_zone_group_member()],
        }
    }

    fn create_test_topology() -> Topology {
        Topology {
            zone_groups: vec![create_test_zone_group()],
            vanished_devices: Some(VanishedDevices {
                devices: vec![VanishedDevice {
                    uuid: "RINCON_VANISHED".to_string(),
                    zone_name: "Old Speaker".to_string(),
                    reason: "powered off".to_string(),
                }],
            }),
        }
    }

    #[test]
    fn test_from_group() {
        let zone_group = create_test_zone_group();

        let group = TopologyItem::from_group(&zone_group);

        assert_eq!(zone_group.id, group.get_uuid());
    }

    #[test]
    fn test_from_speaker() {
        let zone_group_member = create_test_zone_group_member();

        let speaker = TopologyItem::from_speaker(&zone_group_member);

        assert_eq!(zone_group_member.uuid, speaker.get_uuid());
    }

    #[test]
    fn test_from_satellite() {
        let satellite = create_test_satellite();

        let satellite_item = TopologyItem::from_satellite(&satellite);

        assert_eq!(satellite.uuid, satellite_item.get_uuid());
    }

    #[test]
    fn test_to_list_item_group() {
        let group = TopologyItem::Group {
            ip: "192.168.1.100".to_string(),
            name: "Living Room".to_string(),
            uuid: "RINCON_123456".to_string(),
            is_last: false,
            play_state: PlayState::Stopped,
        };

        let list_item = group.to_list_item();
        // Verify the ListItem was created successfully
        drop(list_item);
    }

    #[test]
    fn test_to_list_item_speaker() {
        let speaker = TopologyItem::Speaker {
            ip: "192.168.1.101".to_string(),
            name: "Kitchen".to_string(),
            uuid: "RINCON_789012".to_string(),
            model: Some("Connect:Amp".to_string()),
            is_last: false,
        };

        let list_item = speaker.to_list_item();
        drop(list_item);
    }

    #[test]
    fn test_to_list_item_speaker_last() {
        let speaker = TopologyItem::Speaker {
            ip: "192.168.1.101".to_string(),
            name: "Kitchen".to_string(),
            uuid: "RINCON_789012".to_string(),
            model: Some("Connect:Amp".to_string()),
            is_last: true,
        };

        let list_item = speaker.to_list_item();
        drop(list_item);
    }

    #[test]
    fn test_to_list_item_satellite() {
        let satellite = TopologyItem::Satellite {
            uuid: "RINCON_SAT123".to_string(),
            is_last: false,
        };

        let list_item = satellite.to_list_item();
        drop(list_item);
    }
}
