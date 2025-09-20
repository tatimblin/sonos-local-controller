use super::{get_play_state_icon, TopologyItem};
use crate::topology::justify_content::space_between;
use ratatui::{
    style::{Color, Style},
    text::Span,
    widgets::ListItem,
};
use sonos::{PlayState, SpeakerController, ZoneGroup};

impl TopologyItem {
    pub fn from_group(group: &ZoneGroup) -> Self {
        let ip = group.get_coordinator().get_ip();
        let controller = SpeakerController::new();
        let play_state = controller.get_play_state(&ip).unwrap_or(PlayState::Stopped);
        let volume = controller.get_volume(&ip).ok();

        TopologyItem::Group {
            ip,
            uuid: group.id.to_string(),
            name: group.get_name().to_string(),
            is_last: false,
            play_state,
            volume,
            children_count: group.count_children(),
        }
    }

    pub(super) fn group_to_list_item(&self, _highlighted: bool) -> ListItem<'static> {
        if let TopologyItem::Group {
            name,
            play_state,
            children_count,
            ..
        } = self
        {
            let left_spans = vec![
                Span::raw(get_play_state_icon(play_state)),
                Span::raw(TopologyItem::get_name(name, children_count)),
            ];

            let right_content = Some(Span::styled("Group", Style::default().fg(Color::Blue)));

            let line = space_between(left_spans, right_content);

            ListItem::new(line)
        } else {
            panic!("group_to_list_item called on non-Group variant")
        }
    }

    fn get_name(name: &str, count: &usize) -> String {
      if *count > 1 {
        format!("{} +{}", name, count - 1)
      } else {
        name.to_string()
      }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sonos::{Satellite, ZoneGroupMember};

    fn create_test_zone_group_member() -> ZoneGroupMember {
        ZoneGroupMember {
            uuid: "RINCON_123456".to_string(),
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            zone_name: "Living Room".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![],
        }
    }

    fn create_zone_group() -> ZoneGroup {
        ZoneGroup {
            coordinator: "RINCON_123456".to_string(),
            id: "RINCON_123456:1234567890".to_string(),
            members: vec![create_test_zone_group_member()],
        }
    }

    #[test]
    fn test_from_group() {
        let zone_group = create_zone_group();
        let group = TopologyItem::from_group(&zone_group);
        assert_eq!(zone_group.id, group.get_uuid());
    }

    #[test]
    fn test_to_list_item_group() {
        let group = TopologyItem::Group {
            ip: "192.168.1.100".to_string(),
            name: "Living Room".to_string(),
            uuid: "RINCON_123456".to_string(),
            is_last: false,
            play_state: PlayState::Stopped,
            volume: None,
            children_count: 1,
        };

        let list_item = group.to_list_item(false);

        drop(list_item);
    }
}
