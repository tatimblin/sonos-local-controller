use sonos::Topology;

use crate::topology::topology_item::TopologyItem;

#[derive(Debug, Clone)]
pub struct TopologyList {
	pub items: Vec<TopologyItem>
}

impl TopologyList {
	pub fn new(topology: Topology) -> Self {
		let mut items: Vec<TopologyItem> = Vec::new();

		if topology.len() == 0 {
			return TopologyList {
				items,
			};
		}

		for group in topology.get_groups() {
			let group_item = TopologyItem::from_group(&group);
			items.push(group_item);

			for speaker in group.get_speakers() {
				let speaker_item = TopologyItem::from_speaker(&speaker);
				items.push(speaker_item);
			}
		}

		TopologyList { items }
	}
}
