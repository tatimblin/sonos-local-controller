use super::TopologyItem;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::ListItem,
};
use sonos::Satellite;

impl TopologyItem {
    pub fn from_satellite(satellite: &Satellite) -> Self {
        TopologyItem::Satellite {
            uuid: satellite.uuid.to_string(),
            is_last: false,
        }
    }

    /// Converts a Satellite variant to a ListItem
    pub(super) fn satellite_to_list_item(&self, _highlighted: bool) -> ListItem<'static> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_satellite() -> Satellite {
        Satellite {
            uuid: "RINCON_SAT123".to_string(),
            location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
            zone_name: "Satellite Speaker".to_string(),
            software_version: "56.0-76060".to_string(),
        }
    }

    #[test]
    fn test_from_satellite() {
        let satellite = create_test_satellite();

        let satellite_item = TopologyItem::from_satellite(&satellite);

        assert_eq!(satellite.uuid, satellite_item.get_uuid());
    }

    #[test]
    fn test_to_list_item_satellite() {
        let satellite = TopologyItem::Satellite {
            uuid: "RINCON_SAT123".to_string(),
            is_last: false,
        };

        let list_item = satellite.to_list_item(false);
        drop(list_item);
    }
}
