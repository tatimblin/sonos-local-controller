use sonos::{Satellite, ZoneGroup, ZoneGroupMember};

pub enum TopologyItem {
    Group { uuid: String },
    Speaker { uuid: String },
    Satellite { uuid: String },
}

pub enum TopologyType {
    Group,
    Speaker,
    Satellite,
}

impl TopologyItem {
	pub fn from_group(group: &ZoneGroup) -> Self {
		TopologyItem::Group {
			uuid: group.id.to_string(),
		}
	}

	pub fn from_speaker(speaker: &ZoneGroupMember) -> Self {
		TopologyItem::Speaker {
			uuid: speaker.uuid.to_string(),
		}
	}

	pub fn from_satellite(satellite: &Satellite) -> Self {
		TopologyItem::Satellite {
			uuid: satellite.uuid.to_string(),
		}
	}

	pub fn get_type(&self) -> TopologyType {
		match self {
			TopologyItem::Group { .. } => TopologyType::Group,
			TopologyItem::Speaker { .. } => TopologyType::Speaker,
			TopologyItem::Satellite { .. } => TopologyType::Satellite,
		}
	}
}
