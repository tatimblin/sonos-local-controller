use std::io;
use log::{error, warn, debug};

use sonos::{ SpeakerTrait, System, SystemEvent };

use crate::state::store::Store;
use crate::state::reducers::AppAction;
use crate::topology::topology_list::TopologyList;

pub fn use_speakers(store: &Store, mut render_callback: impl FnMut() -> io::Result<()>) -> io::Result<()> {
  let mut system = System::new()?;

  Ok(for event in system.discover() {
    match event {
      SystemEvent::SpeakerFound(speaker) => {
        store.dispatch(AppAction::SetStatusMessage(speaker.name().to_owned()));
        render_callback()?;
      },
      SystemEvent::TopologyReady(sonos_topology) => {
        debug!("TopologyReady event received");
        let topology = TopologyList::new(sonos_topology);
        store.dispatch(AppAction::SetTopology(topology));

        render_callback()?;
      }
      _ => {}
    }
  })
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use sonos::topology::{Topology as SonosTopology, ZoneGroup, ZoneGroupMember};

//     fn create_test_zone_group_member(uuid: &str, zone_name: &str) -> ZoneGroupMember {
//         ZoneGroupMember {
//             uuid: uuid.to_string(),
//             location: format!("http://192.168.1.100:1400/xml/device_description.xml"),
//             zone_name: zone_name.to_string(),
//             software_version: "56.0-76060".to_string(),
//             configuration: "1".to_string(),
//             icon: "x-rincon-roomicon:living".to_string(),
//             satellites: vec![],
//         }
//     }

//     fn create_test_zone_group(coordinator_uuid: &str, _coordinator_name: &str, members: Vec<(&str, &str)>) -> ZoneGroup {
//         let zone_members: Vec<ZoneGroupMember> = members.iter()
//             .map(|(uuid, name)| create_test_zone_group_member(uuid, name))
//             .collect();

//         ZoneGroup {
//             coordinator: coordinator_uuid.to_string(),
//             id: format!("{}:1234567890", coordinator_uuid),
//             members: zone_members,
//         }
//     }

//     #[test]
//     fn test_transform_topology_single_group() {
//         let sonos_topology = SonosTopology {
//             zone_groups: vec![
//                 create_test_zone_group("RINCON_123", "Living Room", vec![("RINCON_123", "Living Room")])
//             ],
//             vanished_devices: None,
//         };

//         let cli_topology = transform_topology(&sonos_topology).expect("Transformation should succeed");

//         assert_eq!(cli_topology.groups.len(), 1);
//         assert_eq!(cli_topology.groups[0].name, "Living Room");
//         assert_eq!(cli_topology.groups[0].speakers.len(), 1);
//         assert_eq!(cli_topology.groups[0].speakers[0], "Living Room");
//     }

//     #[test]
//     fn test_transform_topology_grouped_speakers() {
//         let sonos_topology = SonosTopology {
//             zone_groups: vec![
//                 create_test_zone_group("RINCON_123", "Living Room", vec![
//                     ("RINCON_123", "Living Room"),
//                     ("RINCON_456", "Kitchen")
//                 ])
//             ],
//             vanished_devices: None,
//         };

//         let cli_topology = transform_topology(&sonos_topology).expect("Transformation should succeed");

//         assert_eq!(cli_topology.groups.len(), 1);
//         assert_eq!(cli_topology.groups[0].name, "Living Room");
//         assert_eq!(cli_topology.groups[0].speakers.len(), 2);
//         assert_eq!(cli_topology.groups[0].speakers[0], "Living Room");
//         assert_eq!(cli_topology.groups[0].speakers[1], "Kitchen");
//     }

//     #[test]
//     fn test_transform_topology_multiple_groups() {
//         let sonos_topology = SonosTopology {
//             zone_groups: vec![
//                 create_test_zone_group("RINCON_123", "Living Room", vec![("RINCON_123", "Living Room")]),
//                 create_test_zone_group("RINCON_456", "Kitchen", vec![("RINCON_456", "Kitchen")])
//             ],
//             vanished_devices: None,
//         };

//         let cli_topology = transform_topology(&sonos_topology).expect("Transformation should succeed");

//         assert_eq!(cli_topology.groups.len(), 2);
//         assert_eq!(cli_topology.groups[0].name, "Living Room");
//         assert_eq!(cli_topology.groups[1].name, "Kitchen");
//     }

//     #[test]
//     fn test_transform_topology_missing_coordinator_fallback() {
//         // Create a zone group where coordinator UUID doesn't match any member
//         let mut zone_group = create_test_zone_group("RINCON_999", "Should Not Be Found", vec![
//             ("RINCON_123", "Living Room"),
//             ("RINCON_456", "Kitchen")
//         ]);
//         zone_group.coordinator = "RINCON_999".to_string(); // Non-existent coordinator

//         let sonos_topology = SonosTopology {
//             zone_groups: vec![zone_group],
//             vanished_devices: None,
//         };

//         let cli_topology = transform_topology(&sonos_topology).expect("Transformation should succeed");

//         assert_eq!(cli_topology.groups.len(), 1);
//         // Should fallback to first member's name
//         assert_eq!(cli_topology.groups[0].name, "Living Room");
//         assert_eq!(cli_topology.groups[0].speakers.len(), 2);
//     }

//     #[test]
//     fn test_transform_topology_empty_group_fallback() {
//         let sonos_topology = SonosTopology {
//             zone_groups: vec![
//                 ZoneGroup {
//                     coordinator: "RINCON_123".to_string(),
//                     id: "RINCON_123:123".to_string(),
//                     members: vec![], // Empty members
//                 }
//             ],
//             vanished_devices: None,
//         };

//         let cli_topology = transform_topology(&sonos_topology).expect("Transformation should succeed");

//         assert_eq!(cli_topology.groups.len(), 1);
//         // Should fallback to "Empty Group 1"
//         assert_eq!(cli_topology.groups[0].name, "Empty Group 1");
//         assert_eq!(cli_topology.groups[0].speakers.len(), 0);
//     }

//     #[test]
//     fn test_transform_topology_empty_topology() {
//         let sonos_topology = SonosTopology {
//             zone_groups: vec![],
//             vanished_devices: None,
//         };

//         let cli_topology = transform_topology(&sonos_topology).expect("Transformation should succeed");

//         assert_eq!(cli_topology.groups.len(), 0);
//     }

//     #[test]
//     fn test_transform_topology_invalid_group_data() {
//         let sonos_topology = SonosTopology {
//             zone_groups: vec![
//                 ZoneGroup {
//                     coordinator: "".to_string(), // Empty coordinator ID
//                     id: "RINCON_123:123".to_string(),
//                     members: vec![create_test_zone_group_member("RINCON_123", "Living Room")],
//                 }
//             ],
//             vanished_devices: None,
//         };

//         let cli_topology = transform_topology(&sonos_topology).expect("Transformation should succeed");

//         // Should skip the invalid group and return empty topology
//         assert_eq!(cli_topology.groups.len(), 0);
//     }

//     #[test]
//     fn test_transform_topology_mixed_valid_invalid_groups() {
//         let sonos_topology = SonosTopology {
//             zone_groups: vec![
//                 // Valid group
//                 create_test_zone_group("RINCON_123", "Living Room", vec![("RINCON_123", "Living Room")]),
//                 // Invalid group with empty coordinator
//                 ZoneGroup {
//                     coordinator: "".to_string(),
//                     id: "RINCON_456:123".to_string(),
//                     members: vec![create_test_zone_group_member("RINCON_456", "Kitchen")],
//                 },
//                 // Another valid group
//                 create_test_zone_group("RINCON_789", "Bedroom", vec![("RINCON_789", "Bedroom")]),
//             ],
//             vanished_devices: None,
//         };

//         let cli_topology = transform_topology(&sonos_topology).expect("Transformation should succeed");

//         // Should only include the valid groups
//         assert_eq!(cli_topology.groups.len(), 2);
//         assert_eq!(cli_topology.groups[0].name, "Living Room");
//         assert_eq!(cli_topology.groups[1].name, "Bedroom");
//     }

//     #[test]
//     fn test_transform_topology_empty_speaker_names() {
//         let sonos_topology = SonosTopology {
//             zone_groups: vec![
//                 ZoneGroup {
//                     coordinator: "RINCON_123".to_string(),
//                     id: "RINCON_123:123".to_string(),
//                     members: vec![
//                         create_test_zone_group_member("RINCON_123", "Living Room"),
//                         create_test_zone_group_member("RINCON_456", ""), // Empty name
//                         create_test_zone_group_member("RINCON_789", "Kitchen"),
//                     ],
//                 }
//             ],
//             vanished_devices: None,
//         };

//         let cli_topology = transform_topology(&sonos_topology).expect("Transformation should succeed");

//         assert_eq!(cli_topology.groups.len(), 1);
//         assert_eq!(cli_topology.groups[0].name, "Living Room");
//         // Should only include speakers with valid names
//         assert_eq!(cli_topology.groups[0].speakers.len(), 2);
//         assert_eq!(cli_topology.groups[0].speakers[0], "Living Room");
//         assert_eq!(cli_topology.groups[0].speakers[1], "Kitchen");
//     }

//     #[test]
//     fn test_transform_topology_empty_group_id() {
//         let sonos_topology = SonosTopology {
//             zone_groups: vec![
//                 ZoneGroup {
//                     coordinator: "RINCON_123".to_string(),
//                     id: "".to_string(), // Empty group ID
//                     members: vec![create_test_zone_group_member("RINCON_123", "Living Room")],
//                 }
//             ],
//             vanished_devices: None,
//         };

//         let cli_topology = transform_topology(&sonos_topology).expect("Transformation should succeed");

//         // Should skip the invalid group and return empty topology
//         assert_eq!(cli_topology.groups.len(), 0);
//     }

//     #[test]
//     fn test_transform_topology_coordinator_with_empty_name_fallback() {
//         // Create a zone group where coordinator exists but has empty zone name
//         let zone_group = create_test_zone_group("RINCON_123", "", vec![
//             ("RINCON_123", ""), // Coordinator with empty name (first member)
//             ("RINCON_456", "Kitchen")
//         ]);

//         let sonos_topology = SonosTopology {
//             zone_groups: vec![zone_group],
//             vanished_devices: None,
//         };

//         let cli_topology = transform_topology(&sonos_topology).expect("Transformation should succeed");

//         assert_eq!(cli_topology.groups.len(), 1);
//         // Should fallback to "Unknown Group 1" since first member (coordinator) has empty name
//         assert_eq!(cli_topology.groups[0].name, "Unknown Group 1");
//         assert_eq!(cli_topology.groups[0].speakers.len(), 1);
//         assert_eq!(cli_topology.groups[0].speakers[0], "Kitchen");
//     }

//     #[test]
//     fn test_transform_topology_all_members_empty_names() {
//         let sonos_topology = SonosTopology {
//             zone_groups: vec![
//                 ZoneGroup {
//                     coordinator: "RINCON_999".to_string(), // Non-existent coordinator
//                     id: "RINCON_999:123".to_string(),
//                     members: vec![
//                         create_test_zone_group_member("RINCON_123", ""), // Empty name
//                         create_test_zone_group_member("RINCON_456", ""), // Empty name
//                     ],
//                 }
//             ],
//             vanished_devices: None,
//         };

//         let cli_topology = transform_topology(&sonos_topology).expect("Transformation should succeed");

//         assert_eq!(cli_topology.groups.len(), 1);
//         // Should fallback to "Unknown Group 1" since no valid names exist
//         assert_eq!(cli_topology.groups[0].name, "Unknown Group 1");
//         assert_eq!(cli_topology.groups[0].speakers.len(), 0); // No valid speaker names
//     }
// }
