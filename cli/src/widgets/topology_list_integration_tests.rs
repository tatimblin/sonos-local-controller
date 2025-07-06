//! Integration tests for TopologyList widget with real topology data
//!
//! These tests verify the widget's behavior with complex multi-group topologies,
//! satellite speakers in home theater setups, and performance with large structures.

use super::topology_list::{TopologyList, HierarchicalItem, ItemType};
use crate::types::{Topology, Group, SpeakerInfo, SonosTopology, ZoneGroup, ZoneGroupMember, Satellite};
use std::time::Instant;

/// Helper function to create a SpeakerInfo from a name
fn create_speaker_info(name: &str, is_coordinator: bool) -> SpeakerInfo {
    SpeakerInfo::from_name(name, is_coordinator)
}

/// Creates a complex multi-group topology with various configurations
/// This simulates a real Sonos system with multiple rooms and groupings
fn create_complex_multi_group_topology() -> Topology {
    Topology {
        groups: vec![
            // Single speaker group
            Group {
                name: "Bedroom".to_string(),
                speakers: vec![create_speaker_info("Bedroom", true)],
            },
            // Multi-speaker group (Living Room + Kitchen)
            Group {
                name: "Living Room".to_string(),
                speakers: vec![
                    create_speaker_info("Living Room", true),
                    create_speaker_info("Kitchen", false),
                ],
            },
            // Another single speaker
            Group {
                name: "Bathroom".to_string(),
                speakers: vec![create_speaker_info("Bathroom", true)],
            },
            // Large group with many speakers
            Group {
                name: "Whole House".to_string(),
                speakers: vec![
                    create_speaker_info("Whole House", true),
                    create_speaker_info("Dining Room", false),
                    create_speaker_info("Office", false),
                    create_speaker_info("Guest Room", false),
                    create_speaker_info("Patio", false),
                ],
            },
        ],
    }
}

/// Creates a SonosTopology with satellite speakers for home theater testing
/// This includes surround speakers and subwoofers typical in home theater setups
fn create_home_theater_topology() -> SonosTopology {
    SonosTopology {
        zone_groups: vec![
            // Home theater setup with satellites
            ZoneGroup {
                coordinator: "RINCON_THEATER_MAIN".to_string(),
                id: "RINCON_THEATER_MAIN:12345".to_string(),
                members: vec![
                    ZoneGroupMember {
                        uuid: "RINCON_THEATER_MAIN".to_string(),
                        location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
                        zone_name: "Living Room".to_string(),
                        software_version: "83.1-62052".to_string(),
                        configuration: "1".to_string(),
                        icon: "x-rincon-roomicon:living".to_string(),
                        satellites: vec![
                            Satellite {
                                uuid: "RINCON_SURROUND_LEFT".to_string(),
                                location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
                                zone_name: "Living Room Left".to_string(),
                                software_version: "83.1-62052".to_string(),
                            },
                            Satellite {
                                uuid: "RINCON_SURROUND_RIGHT".to_string(),
                                location: "http://192.168.1.102:1400/xml/device_description.xml".to_string(),
                                zone_name: "Living Room Right".to_string(),
                                software_version: "83.1-62052".to_string(),
                            },
                            Satellite {
                                uuid: "RINCON_SUBWOOFER".to_string(),
                                location: "http://192.168.1.103:1400/xml/device_description.xml".to_string(),
                                zone_name: "Living Room Sub".to_string(),
                                software_version: "83.1-62052".to_string(),
                            },
                        ],
                    },
                ],
            },
            // Regular speaker without satellites
            ZoneGroup {
                coordinator: "RINCON_BEDROOM".to_string(),
                id: "RINCON_BEDROOM:67890".to_string(),
                members: vec![
                    ZoneGroupMember {
                        uuid: "RINCON_BEDROOM".to_string(),
                        location: "http://192.168.1.104:1400/xml/device_description.xml".to_string(),
                        zone_name: "Bedroom".to_string(),
                        software_version: "83.1-62052".to_string(),
                        configuration: "1".to_string(),
                        icon: "x-rincon-roomicon:bedroom".to_string(),
                        satellites: vec![],
                    },
                ],
            },
            // Grouped speakers with one having satellites
            ZoneGroup {
                coordinator: "RINCON_KITCHEN".to_string(),
                id: "RINCON_KITCHEN:11111".to_string(),
                members: vec![
                    ZoneGroupMember {
                        uuid: "RINCON_KITCHEN".to_string(),
                        location: "http://192.168.1.105:1400/xml/device_description.xml".to_string(),
                        zone_name: "Kitchen".to_string(),
                        software_version: "83.1-62052".to_string(),
                        configuration: "1".to_string(),
                        icon: "x-rincon-roomicon:kitchen".to_string(),
                        satellites: vec![
                            Satellite {
                                uuid: "RINCON_KITCHEN_SATELLITE".to_string(),
                                location: "http://192.168.1.106:1400/xml/device_description.xml".to_string(),
                                zone_name: "Kitchen Satellite".to_string(),
                                software_version: "83.1-62052".to_string(),
                            },
                        ],
                    },
                    ZoneGroupMember {
                        uuid: "RINCON_DINING".to_string(),
                        location: "http://192.168.1.107:1400/xml/device_description.xml".to_string(),
                        zone_name: "Dining Room".to_string(),
                        software_version: "83.1-62052".to_string(),
                        configuration: "1".to_string(),
                        icon: "x-rincon-roomicon:dining".to_string(),
                        satellites: vec![],
                    },
                ],
            },
        ],
        vanished_devices: None,
    }
}

/// Creates a large topology for performance testing
/// Simulates a commercial installation with many speakers
fn create_large_topology() -> Topology {
    let mut groups = Vec::new();
    
    // Create 50 groups with varying sizes
    for i in 1..=50 {
        let group_name = format!("Zone {}", i);
        let mut speakers = vec![create_speaker_info(&group_name, true)];
        
        // Some groups have multiple speakers
        if i % 3 == 0 {
            speakers.push(create_speaker_info(&format!("Zone {} Secondary", i), false));
        }
        if i % 5 == 0 {
            speakers.push(create_speaker_info(&format!("Zone {} Tertiary", i), false));
        }
        if i % 7 == 0 {
            speakers.push(create_speaker_info(&format!("Zone {} Quaternary", i), false));
        }
        
        groups.push(Group {
            name: group_name,
            speakers,
        });
    }
    
    Topology { groups }
}

/// Creates a mixed topology with both grouped and ungrouped speakers
fn create_mixed_grouped_ungrouped_topology() -> Topology {
    Topology {
        groups: vec![
            // Ungrouped speakers (single speaker groups)
            Group {
                name: "Bedroom".to_string(),
                speakers: vec![create_speaker_info("Bedroom", true)],
            },
            Group {
                name: "Bathroom".to_string(),
                speakers: vec![create_speaker_info("Bathroom", true)],
            },
            Group {
                name: "Office".to_string(),
                speakers: vec![create_speaker_info("Office", true)],
            },
            // Grouped speakers
            Group {
                name: "Living Room".to_string(),
                speakers: vec![
                    create_speaker_info("Living Room", true),
                    create_speaker_info("Kitchen", false),
                ],
            },
            Group {
                name: "Master Suite".to_string(),
                speakers: vec![
                    create_speaker_info("Master Suite", true),
                    create_speaker_info("Master Bathroom", false),
                    create_speaker_info("Walk-in Closet", false),
                ],
            },
            // Another ungrouped speaker
            Group {
                name: "Garage".to_string(),
                speakers: vec![create_speaker_info("Garage", true)],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_multi_group_topology_structure() {
        let topology = create_complex_multi_group_topology();
        let topology_list = TopologyList::new(&topology);
        
        // Verify total item count: 4 groups + 9 speakers = 13 items
        // (Bedroom: 1+1, Living Room: 1+2, Bathroom: 1+1, Whole House: 1+5)
        assert_eq!(topology_list.len(), 13);
        
        // Test navigation through all items
        let mut list = topology_list;
        let mut group_count = 0;
        let mut speaker_count = 0;
        
        for i in 0..list.len() {
            // Navigate to each item
            while list.selected().unwrap_or(0) != i {
                list.next();
            }
            
            match list.selected_item_type() {
                Some(ItemType::Group) => group_count += 1,
                Some(ItemType::Speaker) => speaker_count += 1,
                Some(ItemType::Satellite) => panic!("Unexpected satellite in simplified topology"),
                None => panic!("No item selected at index {}", i),
            }
        }
        
        assert_eq!(group_count, 4);
        assert_eq!(speaker_count, 9);
    }

    #[test]
    fn test_complex_multi_group_topology_navigation() {
        let topology = create_complex_multi_group_topology();
        let mut topology_list = TopologyList::new(&topology);
        
        // Test forward navigation through all items
        let mut visited_items = Vec::new();
        for _ in 0..topology_list.len() {
            if let Some(item) = topology_list.selected_item() {
                visited_items.push(item.clone());
            }
            topology_list.next();
        }
        
        // Should have visited all items
        assert_eq!(visited_items.len(), topology_list.len());
        
        // Test backward navigation
        let mut reverse_visited = Vec::new();
        for _ in 0..topology_list.len() {
            topology_list.previous();
            if let Some(item) = topology_list.selected_item() {
                reverse_visited.push(item.clone());
            }
        }
        
        // Should have visited all items in reverse
        assert_eq!(reverse_visited.len(), topology_list.len());
        
        // Verify wrapping behavior
        let first_item = topology_list.selected_item().cloned();
        topology_list.previous(); // Should wrap to last item
        let last_item = topology_list.selected_item().cloned();
        topology_list.next(); // Should wrap back to first item
        let wrapped_first = topology_list.selected_item().cloned();
        
        assert_eq!(first_item, wrapped_first);
        assert_ne!(first_item, last_item);
    }

    #[test]
    fn test_home_theater_satellite_handling() {
        let topology = create_home_theater_topology();
        let topology_list = TopologyList::from_sonos_topology(&topology);
        
        // Count different item types
        let mut group_count = 0;
        let mut speaker_count = 0;
        let mut satellite_count = 0;
        
        let mut list = topology_list;
        for i in 0..list.len() {
            // Navigate to each item
            while list.selected().unwrap_or(0) != i {
                list.next();
            }
            
            match list.selected_item_type() {
                Some(ItemType::Group) => group_count += 1,
                Some(ItemType::Speaker) => speaker_count += 1,
                Some(ItemType::Satellite) => satellite_count += 1,
                None => panic!("No item selected at index {}", i),
            }
        }
        
        // Verify counts: 3 groups + 4 speakers + 4 satellites = 11 items
        assert_eq!(group_count, 3);
        assert_eq!(speaker_count, 4);
        assert_eq!(satellite_count, 4);
        assert_eq!(list.len(), 11);
    }

    #[test]
    fn test_home_theater_satellite_hierarchy() {
        let topology = create_home_theater_topology();
        let mut topology_list = TopologyList::from_sonos_topology(&topology);
        
        // Find the Living Room group and verify its structure
        let mut found_living_room_group = false;
        let mut found_living_room_speaker = false;
        let mut satellite_count_after_living_room = 0;
        
        for _ in 0..topology_list.len() {
            if let Some(item) = topology_list.selected_item() {
                match item {
                    HierarchicalItem::Group { name, member_count } if name == "Living Room" => {
                        found_living_room_group = true;
                        assert_eq!(*member_count, 1); // Only one main speaker
                    }
                    HierarchicalItem::Speaker { name, group_name, .. } 
                        if name == "Living Room" && group_name == "Living Room" => {
                        found_living_room_speaker = true;
                        // Count satellites that follow this speaker
                        let mut temp_list = topology_list.clone();
                        temp_list.next();
                        while let Some(next_item) = temp_list.selected_item() {
                            match next_item {
                                HierarchicalItem::Satellite { parent_speaker_name, .. } 
                                    if parent_speaker_name == "Living Room" => {
                                    satellite_count_after_living_room += 1;
                                    temp_list.next();
                                }
                                _ => break,
                            }
                        }
                    }
                    _ => {}
                }
            }
            topology_list.next();
        }
        
        assert!(found_living_room_group, "Living Room group not found");
        assert!(found_living_room_speaker, "Living Room speaker not found");
        assert_eq!(satellite_count_after_living_room, 3, "Expected 3 satellites after Living Room speaker");
    }

    #[test]
    fn test_mixed_grouped_ungrouped_display() {
        let topology = create_mixed_grouped_ungrouped_topology();
        let topology_list = TopologyList::new(&topology);
        
        // Verify structure: 6 groups + 9 speakers = 15 items
        // (4 single + 2 multi groups, with 9 total speakers)
        assert_eq!(topology_list.len(), 15);
        
        let mut list = topology_list;
        let mut single_speaker_groups = 0;
        let mut multi_speaker_groups = 0;
        
        for i in 0..list.len() {
            while list.selected().unwrap_or(0) != i {
                list.next();
            }
            
            if let Some(HierarchicalItem::Group { member_count, .. }) = list.selected_item() {
                if *member_count == 1 {
                    single_speaker_groups += 1;
                } else {
                    multi_speaker_groups += 1;
                }
            }
        }
        
        assert_eq!(single_speaker_groups, 4); // Bedroom, Bathroom, Office, Garage
        assert_eq!(multi_speaker_groups, 2);  // Living Room group, Master Suite group
    }

    #[test]
    fn test_large_topology_performance() {
        let start_time = Instant::now();
        
        // Create large topology
        let topology = create_large_topology();
        let creation_time = start_time.elapsed();
        
        // Create widget
        let widget_start = Instant::now();
        let topology_list = TopologyList::new(&topology);
        let widget_creation_time = widget_start.elapsed();
        
        // Test navigation performance
        let nav_start = Instant::now();
        let mut list = topology_list;
        
        // Navigate through all items multiple times
        for _ in 0..5 {
            for _ in 0..list.len() {
                list.next();
                // Access selected item to ensure it's computed
                let _ = list.selected_item();
            }
        }
        let navigation_time = nav_start.elapsed();
        
        // Performance assertions (generous limits for CI environments)
        assert!(creation_time.as_millis() < 100, "Topology creation took too long: {:?}", creation_time);
        assert!(widget_creation_time.as_millis() < 50, "Widget creation took too long: {:?}", widget_creation_time);
        assert!(navigation_time.as_millis() < 200, "Navigation took too long: {:?}", navigation_time);
        
        // Verify correctness wasn't sacrificed for performance
        assert!(list.len() > 100, "Large topology should have many items");
        assert!(list.selected_item().is_some(), "Should have valid selection");
    }

    #[test]
    fn test_large_topology_memory_usage() {
        let topology = create_large_topology();
        let topology_list = TopologyList::new(&topology);
        
        // Verify the widget handles large topologies without excessive memory usage
        // by checking that all items are accessible and properly structured
        let mut unique_groups = std::collections::HashSet::new();
        let mut unique_speakers = std::collections::HashSet::new();
        
        let mut list = topology_list;
        for i in 0..list.len() {
            while list.selected().unwrap_or(0) != i {
                list.next();
            }
            
            match list.selected_item() {
                Some(HierarchicalItem::Group { name, .. }) => {
                    unique_groups.insert(name.clone());
                }
                Some(HierarchicalItem::Speaker { name, .. }) => {
                    unique_speakers.insert(name.clone());
                }
                _ => {}
            }
        }
        
        // Verify we have the expected number of unique items
        assert_eq!(unique_groups.len(), 50, "Should have 50 unique groups");
        assert!(unique_speakers.len() >= 50, "Should have at least 50 unique speakers");
    }

    #[test]
    fn test_real_topology_data_parsing() {
        // Test with a realistic topology structure similar to the test data
        let realistic_topology = Topology {
            groups: vec![
                Group {
                    name: "Roam 2".to_string(),
                    speakers: vec![create_speaker_info("Roam 2", true)],
                },
                Group {
                    name: "Bathroom".to_string(),
                    speakers: vec![create_speaker_info("Bathroom", true)],
                },
                Group {
                    name: "Kitchen".to_string(),
                    speakers: vec![
                        create_speaker_info("Kitchen", true),
                        create_speaker_info("Living Room", false),
                    ],
                },
                Group {
                    name: "Bedroom".to_string(),
                    speakers: vec![create_speaker_info("Bedroom", true)],
                },
            ],
        };
        
        let topology_list = TopologyList::new(&realistic_topology);
        
        // Verify structure matches expected real-world data
        assert_eq!(topology_list.len(), 9); // 4 groups + 5 speakers
        
        // Test that we can navigate and access all items
        let mut list = topology_list;
        let mut items_found = Vec::new();
        
        for _ in 0..list.len() {
            if let Some(item) = list.selected_item() {
                items_found.push(item.clone());
            }
            list.next();
        }
        
        assert_eq!(items_found.len(), 9);
        
        // Verify we have the expected groups
        let group_names: Vec<String> = items_found.iter()
            .filter_map(|item| match item {
                HierarchicalItem::Group { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect();
        
        assert!(group_names.contains(&"Roam 2".to_string()));
        assert!(group_names.contains(&"Bathroom".to_string()));
        assert!(group_names.contains(&"Kitchen".to_string()));
        assert!(group_names.contains(&"Bedroom".to_string()));
    }

    #[test]
    fn test_edge_case_empty_topology() {
        let empty_topology = Topology { groups: vec![] };
        let topology_list = TopologyList::new(&empty_topology);
        
        assert_eq!(topology_list.len(), 0);
        assert!(topology_list.is_empty());
        assert!(topology_list.selected_item().is_none());
        assert!(topology_list.selected_item_type().is_none());
    }

    #[test]
    fn test_edge_case_single_speaker_system() {
        let single_speaker_topology = Topology {
            groups: vec![
                Group {
                    name: "Only Speaker".to_string(),
                    speakers: vec![create_speaker_info("Only Speaker", true)],
                },
            ],
        };
        
        let mut topology_list = TopologyList::new(&single_speaker_topology);
        
        assert_eq!(topology_list.len(), 2); // 1 group + 1 speaker
        
        // Test navigation wrapping with minimal items
        let first_item = topology_list.selected_item().cloned();
        topology_list.next();
        let second_item = topology_list.selected_item().cloned();
        topology_list.next(); // Should wrap to first
        let wrapped_first = topology_list.selected_item().cloned();
        
        assert_eq!(first_item, wrapped_first);
        assert_ne!(first_item, second_item);
    }

    #[test]
    fn test_satellite_speaker_integration() {
        let topology = create_home_theater_topology();
        let mut topology_list = TopologyList::from_sonos_topology(&topology);
        
        // Find a satellite and verify its properties
        let mut found_satellite = false;
        
        for _ in 0..topology_list.len() {
            if let Some(HierarchicalItem::Satellite { name, parent_speaker_name, group_name }) = topology_list.selected_item() {
                found_satellite = true;
                
                // Verify satellite has proper parent relationship
                assert!(!parent_speaker_name.is_empty(), "Satellite should have parent speaker");
                assert!(!group_name.is_empty(), "Satellite should have group name");
                assert!(name.contains("Left") || name.contains("Right") || name.contains("Sub"), 
                    "Satellite name should indicate its type: {}", name);
                
                break;
            }
            topology_list.next();
        }
        
        assert!(found_satellite, "Should find at least one satellite speaker");
    }

    #[test]
    fn test_coordinator_speaker_ordering() {
        let topology = create_complex_multi_group_topology();
        let mut topology_list = TopologyList::new(&topology);
        
        // Find the "Whole House" group and verify coordinator comes first
        let mut found_whole_house_group = false;
        
        for _ in 0..topology_list.len() {
            if let Some(HierarchicalItem::Group { name, .. }) = topology_list.selected_item() {
                if name == "Whole House" {
                    found_whole_house_group = true;
                    
                    // Next item should be the coordinator speaker
                    topology_list.next();
                    if let Some(HierarchicalItem::Speaker { name, is_coordinator, .. }) = topology_list.selected_item() {
                        assert_eq!(name, "Whole House", "First speaker should be coordinator");
                        assert!(*is_coordinator, "First speaker should be marked as coordinator");
                    } else {
                        panic!("Expected speaker after group");
                    }
                    
                    break;
                }
            }
            topology_list.next();
        }
        
        assert!(found_whole_house_group, "Should find Whole House group");
    }

    #[test]
    fn test_stress_test_with_extreme_topology() {
        // Create an extremely large topology to test performance limits
        let mut groups = Vec::new();
        
        // Create 200 groups with varying configurations
        for i in 1..=200 {
            let group_name = format!("Zone {}", i);
            let mut speakers = vec![create_speaker_info(&group_name, true)];
            
            // Create groups with different sizes to test various scenarios
            match i % 10 {
                0 => {
                    // Large groups (10 speakers)
                    for j in 1..10 {
                        speakers.push(create_speaker_info(&format!("Zone {} Speaker {}", i, j), false));
                    }
                }
                5 => {
                    // Medium groups (5 speakers)
                    for j in 1..5 {
                        speakers.push(create_speaker_info(&format!("Zone {} Speaker {}", i, j), false));
                    }
                }
                2 | 4 | 6 | 8 => {
                    // Small groups (2 speakers)
                    speakers.push(create_speaker_info(&format!("Zone {} Secondary", i), false));
                }
                _ => {
                    // Single speaker groups (already have coordinator)
                }
            }
            
            groups.push(Group {
                name: group_name,
                speakers,
            });
        }
        
        let large_topology = Topology { groups };
        
        // Test creation performance
        let start_time = Instant::now();
        let topology_list = TopologyList::new(&large_topology);
        let creation_time = start_time.elapsed();
        
        // Should handle creation of very large topologies quickly
        assert!(creation_time.as_millis() < 200, "Large topology creation took too long: {:?}", creation_time);
        
        // Verify the structure is correct
        assert!(topology_list.len() > 500, "Should have many items in large topology");
        
        // Test navigation performance under stress
        let nav_start = Instant::now();
        let mut list = topology_list;
        
        // Perform intensive navigation operations
        for _ in 0..10 {
            // Navigate through entire list
            for _ in 0..list.len() {
                list.next();
                // Access item properties to ensure computation happens
                if let Some(item) = list.selected_item() {
                    match item {
                        HierarchicalItem::Group { name, member_count } => {
                            assert!(!name.is_empty());
                            assert!(*member_count > 0);
                        }
                        HierarchicalItem::Speaker { name, group_name, .. } => {
                            assert!(!name.is_empty());
                            assert!(!group_name.is_empty());
                        }
                        HierarchicalItem::Satellite { .. } => {
                            // Shouldn't have satellites in simplified topology
                            panic!("Unexpected satellite in simplified topology");
                        }
                    }
                }
            }
            
            // Navigate backwards
            for _ in 0..list.len() {
                list.previous();
                let _ = list.selected_item(); // Ensure computation
            }
        }
        
        let navigation_time = nav_start.elapsed();
        
        // Navigation should remain fast even with large datasets
        assert!(navigation_time.as_millis() < 500, "Navigation took too long: {:?}", navigation_time);
        
        // Test selection consistency
        let first_item = list.selected_item().cloned();
        for _ in 0..list.len() {
            list.next();
        }
        let wrapped_item = list.selected_item().cloned();
        assert_eq!(first_item, wrapped_item, "Selection should wrap correctly");
    }

    #[test]
    fn test_complex_sonos_topology_with_mixed_satellites() {
        // Create a complex SonosTopology that mixes different satellite configurations
        let complex_topology = SonosTopology {
            zone_groups: vec![
                // Group 1: Single speaker with multiple satellites (5.1 setup)
                ZoneGroup {
                    coordinator: "RINCON_THEATER".to_string(),
                    id: "RINCON_THEATER:1".to_string(),
                    members: vec![
                        ZoneGroupMember {
                            uuid: "RINCON_THEATER".to_string(),
                            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
                            zone_name: "Home Theater".to_string(),
                            software_version: "83.1-62052".to_string(),
                            configuration: "1".to_string(),
                            icon: "x-rincon-roomicon:theater".to_string(),
                            satellites: vec![
                                Satellite {
                                    uuid: "RINCON_SURROUND_L".to_string(),
                                    location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
                                    zone_name: "Theater Left Surround".to_string(),
                                    software_version: "83.1-62052".to_string(),
                                },
                                Satellite {
                                    uuid: "RINCON_SURROUND_R".to_string(),
                                    location: "http://192.168.1.102:1400/xml/device_description.xml".to_string(),
                                    zone_name: "Theater Right Surround".to_string(),
                                    software_version: "83.1-62052".to_string(),
                                },
                                Satellite {
                                    uuid: "RINCON_SUB".to_string(),
                                    location: "http://192.168.1.103:1400/xml/device_description.xml".to_string(),
                                    zone_name: "Theater Subwoofer".to_string(),
                                    software_version: "83.1-62052".to_string(),
                                },
                            ],
                        },
                    ],
                },
                // Group 2: Multiple speakers, some with satellites
                ZoneGroup {
                    coordinator: "RINCON_KITCHEN".to_string(),
                    id: "RINCON_KITCHEN:2".to_string(),
                    members: vec![
                        ZoneGroupMember {
                            uuid: "RINCON_KITCHEN".to_string(),
                            location: "http://192.168.1.104:1400/xml/device_description.xml".to_string(),
                            zone_name: "Kitchen".to_string(),
                            software_version: "83.1-62052".to_string(),
                            configuration: "1".to_string(),
                            icon: "x-rincon-roomicon:kitchen".to_string(),
                            satellites: vec![
                                Satellite {
                                    uuid: "RINCON_KITCHEN_SAT".to_string(),
                                    location: "http://192.168.1.105:1400/xml/device_description.xml".to_string(),
                                    zone_name: "Kitchen Satellite".to_string(),
                                    software_version: "83.1-62052".to_string(),
                                },
                            ],
                        },
                        ZoneGroupMember {
                            uuid: "RINCON_DINING".to_string(),
                            location: "http://192.168.1.106:1400/xml/device_description.xml".to_string(),
                            zone_name: "Dining Room".to_string(),
                            software_version: "83.1-62052".to_string(),
                            configuration: "1".to_string(),
                            icon: "x-rincon-roomicon:dining".to_string(),
                            satellites: vec![], // No satellites
                        },
                        ZoneGroupMember {
                            uuid: "RINCON_PATIO".to_string(),
                            location: "http://192.168.1.107:1400/xml/device_description.xml".to_string(),
                            zone_name: "Patio".to_string(),
                            software_version: "83.1-62052".to_string(),
                            configuration: "1".to_string(),
                            icon: "x-rincon-roomicon:patio".to_string(),
                            satellites: vec![
                                Satellite {
                                    uuid: "RINCON_PATIO_SAT1".to_string(),
                                    location: "http://192.168.1.108:1400/xml/device_description.xml".to_string(),
                                    zone_name: "Patio Left".to_string(),
                                    software_version: "83.1-62052".to_string(),
                                },
                                Satellite {
                                    uuid: "RINCON_PATIO_SAT2".to_string(),
                                    location: "http://192.168.1.109:1400/xml/device_description.xml".to_string(),
                                    zone_name: "Patio Right".to_string(),
                                    software_version: "83.1-62052".to_string(),
                                },
                            ],
                        },
                    ],
                },
                // Group 3: Single speaker, no satellites
                ZoneGroup {
                    coordinator: "RINCON_BEDROOM".to_string(),
                    id: "RINCON_BEDROOM:3".to_string(),
                    members: vec![
                        ZoneGroupMember {
                            uuid: "RINCON_BEDROOM".to_string(),
                            location: "http://192.168.1.110:1400/xml/device_description.xml".to_string(),
                            zone_name: "Bedroom".to_string(),
                            software_version: "83.1-62052".to_string(),
                            configuration: "1".to_string(),
                            icon: "x-rincon-roomicon:bedroom".to_string(),
                            satellites: vec![],
                        },
                    ],
                },
            ],
            vanished_devices: None,
        };
        
        let topology_list = TopologyList::from_sonos_topology(&complex_topology);
        
        // Expected structure:
        // Group 1: Home Theater (1 speaker, 3 satellites) = 1 + 1 + 3 = 5 items
        // Group 2: Kitchen group (3 speakers, 3 satellites total) = 1 + 3 + 3 = 7 items  
        // Group 3: Bedroom (1 speaker, 0 satellites) = 1 + 1 = 2 items
        // Total: 5 + 7 + 2 = 14 items
        assert_eq!(topology_list.len(), 14);
        
        // Verify satellite hierarchy and parent relationships
        let mut list = topology_list;
        let mut satellite_parent_relationships = Vec::new();
        
        for _ in 0..list.len() {
            if let Some(HierarchicalItem::Satellite { name, parent_speaker_name, group_name }) = list.selected_item() {
                satellite_parent_relationships.push((name.clone(), parent_speaker_name.clone(), group_name.clone()));
            }
            list.next();
        }
        
        // Should have 6 satellites total (3 + 1 + 2)
        assert_eq!(satellite_parent_relationships.len(), 6);
        
        // Verify specific parent relationships
        let theater_satellites: Vec<_> = satellite_parent_relationships.iter()
            .filter(|(_, parent, group)| parent == "Home Theater" && group == "Home Theater")
            .collect();
        assert_eq!(theater_satellites.len(), 3, "Home Theater should have 3 satellites");
        
        let kitchen_satellites: Vec<_> = satellite_parent_relationships.iter()
            .filter(|(_, parent, group)| parent == "Kitchen" && group == "Kitchen")
            .collect();
        assert_eq!(kitchen_satellites.len(), 1, "Kitchen should have 1 satellite");
        
        let patio_satellites: Vec<_> = satellite_parent_relationships.iter()
            .filter(|(_, parent, group)| parent == "Patio" && group == "Kitchen")
            .collect();
        assert_eq!(patio_satellites.len(), 2, "Patio should have 2 satellites");
    }
}