use super::TopologyItem;
use crate::topology::justify_content::space_between;
use ratatui::{
    style::{Color, Style},
    text::Span,
    widgets::ListItem,
};
use sonos::{SpeakerController, ZoneGroupMember};

impl TopologyItem {
  pub fn from_speaker(coordinator_ip: &str, group_uuid: &str, speaker: &ZoneGroupMember) -> Self {
    let ip = speaker.get_ip();
    let controller = SpeakerController::new();
    let volume = controller.get_volume(&ip).ok();

    TopologyItem::Speaker {
      ip,
      coordinator_ip: coordinator_ip.to_string(),
      name: speaker.zone_name.to_string(),
      group_uuid: format!("GROUP:{}", group_uuid.to_string()),
      uuid: speaker.uuid.to_string(),
      model: None,
      is_last: false,
      volume,
    }
  }

  pub(super) fn speaker_to_list_item(&self, highlighted: bool) -> ListItem<'static> {
    if let TopologyItem::Speaker {
      name,
      model,
      is_last,
      volume,
      ..
    } = self
    {
      let prefix = if *is_last { "└─ " } else { "├─ " };
      let mut left_spans = vec![
        Span::raw("  "),
        Span::raw(prefix),
        Span::raw(name.clone())
      ];

      if let Some(model_name) = model {
        let style = if highlighted {
          Style::default()
        } else {
          Style::default().fg(Color::Gray)
        };
        left_spans.push(Span::styled(" • ", style));
        left_spans.push(Span::styled(model_name.clone(), style));
      }

      let right_content = volume.as_ref().map(|v| Span::raw(format!("{}%", v)));
      
      let line = space_between(left_spans, right_content);

      ListItem::new(line)
    } else {
      panic!("speaker_to_list_item called on non-Speaker variant")
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use sonos::Satellite;

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

  #[test]
  fn test_from_speaker() {
    let zone_group_member = create_test_zone_group_member();

    let speaker = TopologyItem::from_speaker("10.0.0.1", "10.0.0.1", &zone_group_member);

    assert_eq!(zone_group_member.uuid, speaker.get_uuid());
  }

  #[test]
  fn test_to_list_item_speaker() {
    let speaker = TopologyItem::Speaker {
      ip: "192.168.1.101".to_string(),
      coordinator_ip: "10.0.0.1".to_string(),
      group_uuid: "RINCON_789012:123".to_string(),
      name: "Kitchen".to_string(),
      uuid: "RINCON_789012".to_string(),
      model: Some("Connect:Amp".to_string()),
      is_last: false,
      volume: Some(10),
    };

    let list_item = speaker.to_list_item(false);

    drop(list_item);
  }

  #[test]
  fn test_to_list_item_speaker_last() {
    let speaker = TopologyItem::Speaker {
      ip: "192.168.1.101".to_string(),
      coordinator_ip: "10.0.0.1".to_string(),
      group_uuid: "RINCON_789012:123".to_string(),
      name: "Kitchen".to_string(),
      uuid: "RINCON_789012".to_string(),
      model: Some("Connect:Amp".to_string()),
      is_last: true,
      volume: Some(10),
    };

    let list_item = speaker.to_list_item(false);
    drop(list_item);
  }
}
