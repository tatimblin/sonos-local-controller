use sonos::Topology;

use crate::topology::topology_item::TopologyItem;

struct TopologyList {
	items: Vec<TopologyItem>
}

impl TopologyList {
	fn new(topology: Topology) -> Self {
		let mut items: Vec<TopologyItem> = Vec::new();

		if topology.zone_group_count() == 0 {
			return TopologyList {
				items,
			};
		}

		for group in topology.zone_groups {
			let group_item = TopologyItem::from_group(&group);
			items.push(group_item);

			let speakers = group.speakers;
		}

		TopologyList { items }
	}
}
