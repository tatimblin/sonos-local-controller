use crate::types::{Topology, Group, SonosTopology, ZoneGroup};
use super::selectable_list::SelectableList;
use ratatui::{layout::Rect, Frame};

/// Utility for flattening topology structure into a navigable hierarchical list
struct TopologyFlattener;

impl TopologyFlattener {
    /// Flattens a simplified topology into a list of hierarchical items with display strings
    /// 
    /// Returns a tuple of (items, display_strings) where items can be navigated
    /// and display_strings provide the formatted text for rendering
    /// 
    /// # Edge Case Handling
    /// - Empty topology: Returns empty vectors
    /// - Groups with no speakers: Skipped with debug warning
    /// - Empty speaker names: Replaced with placeholder text
    /// - Missing coordinator: Uses first speaker or group name as fallback
    fn flatten(topology: &Topology) -> (Vec<HierarchicalItem>, Vec<String>) {
        let mut items = Vec::new();
        let mut display_strings = Vec::new();

        // Handle empty topology gracefully
        if topology.groups.is_empty() {
            #[cfg(debug_assertions)]
            eprintln!("Warning: Empty topology provided to TopologyFlattener::flatten");
            return (items, display_strings);
        }

        for group in &topology.groups {
            // Validate group data and skip malformed groups
            if group.speakers.is_empty() {
                #[cfg(debug_assertions)]
                eprintln!("Warning: Skipping group '{}' with no speakers", group.name);
                continue;
            }

            // Handle empty or invalid group names
            let group_name = if group.name.trim().is_empty() {
                "Unknown Group".to_string()
            } else {
                group.name.clone()
            };

            // Add the group itself
            let group_item = HierarchicalItem::Group {
                name: group_name.clone(),
                member_count: group.speakers.len(),
            };
            let group_display = Self::format_group_safe(group, &group_name);
            items.push(group_item);
            display_strings.push(group_display);

            // Handle missing coordinator - find coordinator or use fallback
            let coordinator_name = Self::find_coordinator_or_fallback(group);
            
            // Add member speakers - coordinator is first, then others alphabetically
            let mut speakers = group.speakers.clone();
            speakers.sort_by(|a, b| {
                // Coordinator comes first
                if a == &coordinator_name {
                    std::cmp::Ordering::Less
                } else if b == &coordinator_name {
                    std::cmp::Ordering::Greater
                } else {
                    // Non-coordinators sorted alphabetically
                    a.cmp(b)
                }
            });

            for speaker_name in &speakers {
                // Handle empty speaker names
                let safe_speaker_name = if speaker_name.trim().is_empty() {
                    "Unknown Speaker".to_string()
                } else {
                    speaker_name.clone()
                };

                let is_coordinator = speaker_name == &coordinator_name;
                let speaker_item = HierarchicalItem::from_speaker(&safe_speaker_name, &group_name, is_coordinator);
                let speaker_display = Self::format_speaker(&safe_speaker_name, 1);
                items.push(speaker_item);
                display_strings.push(speaker_display);
            }
        }

        (items, display_strings)
    }

    /// Flattens a full SonosTopology into a list of hierarchical items with display strings
    /// This version supports satellites and provides enhanced visual hierarchy
    /// 
    /// Returns a tuple of (items, display_strings) where items can be navigated
    /// and display_strings provide the formatted text for rendering
    /// 
    /// # Edge Case Handling
    /// - Empty topology: Returns empty vectors
    /// - Zone groups with no members: Skipped with debug warning
    /// - Missing coordinator speakers: Uses group ID or first member as fallback
    /// - Empty zone names: Replaced with placeholder text
    /// - Malformed UUIDs: Handled gracefully with warnings
    fn flatten_sonos_topology(topology: &SonosTopology) -> (Vec<HierarchicalItem>, Vec<String>) {
        let mut items = Vec::new();
        let mut display_strings = Vec::new();

        // Handle empty topology gracefully
        if topology.zone_groups.is_empty() {
            #[cfg(debug_assertions)]
            eprintln!("Warning: Empty SonosTopology provided to TopologyFlattener::flatten_sonos_topology");
            return (items, display_strings);
        }

        for zone_group in &topology.zone_groups {
            // Validate zone group data and skip malformed groups
            if zone_group.members.is_empty() {
                #[cfg(debug_assertions)]
                eprintln!("Warning: Skipping zone group '{}' with no members", zone_group.id);
                continue;
            }

            // Handle missing coordinator speaker gracefully
            let coordinator = zone_group.coordinator_speaker();
            let coordinator_name = match coordinator {
                Some(c) => {
                    if c.zone_name.trim().is_empty() {
                        #[cfg(debug_assertions)]
                        eprintln!("Warning: Coordinator speaker has empty zone name, using fallback");
                        Self::extract_fallback_name_from_group_id(&zone_group.id)
                    } else {
                        c.zone_name.clone()
                    }
                }
                None => {
                    #[cfg(debug_assertions)]
                    eprintln!("Warning: No coordinator speaker found for group '{}', using fallback", zone_group.id);
                    
                    // Try to use first member as fallback
                    if let Some(first_member) = zone_group.members.first() {
                        if !first_member.zone_name.trim().is_empty() {
                            first_member.zone_name.clone()
                        } else {
                            Self::extract_fallback_name_from_group_id(&zone_group.id)
                        }
                    } else {
                        Self::extract_fallback_name_from_group_id(&zone_group.id)
                    }
                }
            };

            // Add the zone group itself
            let group_item = HierarchicalItem::Group {
                name: coordinator_name.clone(),
                member_count: zone_group.members.len(),
            };
            let group_display = Self::format_zone_group_safe(zone_group, &coordinator_name);
            items.push(group_item);
            display_strings.push(group_display);

            // Sort members: coordinator first, then others alphabetically
            let mut members = zone_group.members.clone();
            members.sort_by(|a, b| {
                if a.uuid == zone_group.coordinator {
                    std::cmp::Ordering::Less
                } else if b.uuid == zone_group.coordinator {
                    std::cmp::Ordering::Greater
                } else {
                    a.zone_name.cmp(&b.zone_name)
                }
            });

            for member in &members {
                // Handle empty member zone names
                let safe_member_name = if member.zone_name.trim().is_empty() {
                    format!("Unknown Speaker ({})", &member.uuid[..8.min(member.uuid.len())])
                } else {
                    member.zone_name.clone()
                };

                let is_coordinator = member.uuid == zone_group.coordinator;
                let speaker_item = HierarchicalItem::Speaker {
                    name: safe_member_name.clone(),
                    group_name: coordinator_name.clone(),
                    is_coordinator,
                };
                let speaker_display = Self::format_speaker(&safe_member_name, 1);
                items.push(speaker_item);
                display_strings.push(speaker_display);

                // Add satellites for this speaker
                for satellite in &member.satellites {
                    // Handle empty satellite zone names
                    let safe_satellite_name = if satellite.zone_name.trim().is_empty() {
                        format!("Unknown Satellite ({})", &satellite.uuid[..8.min(satellite.uuid.len())])
                    } else {
                        satellite.zone_name.clone()
                    };

                    let satellite_item = HierarchicalItem::Satellite {
                        name: safe_satellite_name.clone(),
                        parent_speaker_name: safe_member_name.clone(),
                        group_name: coordinator_name.clone(),
                    };
                    let satellite_display = Self::format_satellite(&safe_satellite_name, 2);
                    items.push(satellite_item);
                    display_strings.push(satellite_display);
                }
            }
        }

        (items, display_strings)
    }

    /// Formats a simplified group for display with coordinator name and member count
    fn format_group(group: &Group) -> String {
        let member_text = if group.speakers.len() == 1 { "speaker" } else { "speakers" };
        format!("Group: {} ({} {})", group.name, group.speakers.len(), member_text)
    }

    /// Formats a zone group for display with coordinator name and member count
    /// Provides enhanced formatting for full topology data
    fn format_zone_group(zone_group: &ZoneGroup, coordinator_name: &str) -> String {
        let member_text = if zone_group.members.len() == 1 { "speaker" } else { "speakers" };
        let total_speakers = zone_group.total_speaker_count();
        
        if total_speakers > zone_group.members.len() {
            // Include satellite count in display
            let satellite_count = total_speakers - zone_group.members.len();
            let satellite_text = if satellite_count == 1 { "satellite" } else { "satellites" };
            format!("Group: {} ({} {}, {} {})", 
                coordinator_name, 
                zone_group.members.len(), 
                member_text,
                satellite_count,
                satellite_text
            )
        } else {
            format!("Group: {} ({} {})", coordinator_name, zone_group.members.len(), member_text)
        }
    }

    /// Formats a speaker for display with the specified indentation level
    /// Uses consistent 2-space indentation for visual hierarchy
    fn format_speaker(speaker_name: &str, indent_level: usize) -> String {
        let indent = "  ".repeat(indent_level);
        format!("{}Speaker: {}", indent, speaker_name)
    }

    /// Formats a satellite speaker for display with deeper indentation
    /// Uses 4-space indentation (2 levels) to show sub-hierarchy under speakers
    fn format_satellite(satellite_name: &str, indent_level: usize) -> String {
        let indent = "  ".repeat(indent_level);
        format!("{}Satellite: {}", indent, satellite_name)
    }

    /// Helper method to find coordinator speaker or provide fallback
    /// Handles cases where the coordinator speaker is missing or invalid
    fn find_coordinator_or_fallback(group: &Group) -> String {
        // First try to find the coordinator by matching group name
        if group.speakers.iter().any(|s| s == &group.name) {
            return group.name.clone();
        }

        // If no exact match, try case-insensitive match
        for speaker in &group.speakers {
            if speaker.to_lowercase() == group.name.to_lowercase() {
                return speaker.clone();
            }
        }

        // If still no match, use the first speaker as fallback
        if let Some(first_speaker) = group.speakers.first() {
            #[cfg(debug_assertions)]
            eprintln!("Warning: Coordinator '{}' not found in speakers, using '{}' as fallback", 
                group.name, first_speaker);
            return first_speaker.clone();
        }

        // Final fallback to group name (should not happen due to earlier validation)
        group.name.clone()
    }

    /// Extracts a fallback name from a zone group ID
    /// Used when coordinator speaker is missing or has empty name
    fn extract_fallback_name_from_group_id(group_id: &str) -> String {
        // Try to extract meaningful name from group ID like "RINCON_123456:1234567890"
        if let Some(colon_pos) = group_id.find(':') {
            let uuid_part = &group_id[..colon_pos];
            if uuid_part.starts_with("RINCON_") {
                format!("Group ({})", &uuid_part[7..15.min(uuid_part.len())])
            } else {
                format!("Group ({})", &uuid_part[..8.min(uuid_part.len())])
            }
        } else {
            format!("Group ({})", &group_id[..8.min(group_id.len())])
        }
    }

    /// Safe version of format_group that handles edge cases
    fn format_group_safe(group: &Group, safe_name: &str) -> String {
        let member_count = group.speakers.len();
        let member_text = if member_count == 1 { "speaker" } else { "speakers" };
        format!("Group: {} ({} {})", safe_name, member_count, member_text)
    }

    /// Safe version of format_zone_group that handles edge cases
    fn format_zone_group_safe(zone_group: &ZoneGroup, safe_coordinator_name: &str) -> String {
        let member_count = zone_group.members.len();
        let member_text = if member_count == 1 { "speaker" } else { "speakers" };
        let total_speakers = zone_group.total_speaker_count();
        
        if total_speakers > member_count {
            // Include satellite count in display
            let satellite_count = total_speakers - member_count;
            let satellite_text = if satellite_count == 1 { "satellite" } else { "satellites" };
            format!("Group: {} ({} {}, {} {})", 
                safe_coordinator_name, 
                member_count, 
                member_text,
                satellite_count,
                satellite_text
            )
        } else {
            format!("Group: {} ({} {})", safe_coordinator_name, member_count, member_text)
        }
    }
}

/// Represents the different types of items that can be selected in the hierarchical list
#[derive(Debug, Clone, PartialEq)]
pub enum HierarchicalItem {
    /// A group containing one or more speakers
    Group {
        /// Name of the coordinator speaker for this group
        name: String,
        /// Number of member speakers in this group
        member_count: usize,
    },
    /// A speaker in the Sonos system
    Speaker {
        /// Human-readable name for this speaker
        name: String,
        /// Name of the group this speaker belongs to
        group_name: String,
        /// Whether this speaker is the coordinator of its group
        is_coordinator: bool,
    },
    /// A satellite speaker (e.g., surround speakers in home theater setup)
    Satellite {
        /// Human-readable name for this satellite speaker
        name: String,
        /// Name of the parent speaker this satellite belongs to
        parent_speaker_name: String,
        /// Name of the group this satellite's parent belongs to
        group_name: String,
    },
}

/// Enum for identifying the type of a hierarchical item
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemType {
    /// Item is a group
    Group,
    /// Item is a speaker
    Speaker,
    /// Item is a satellite speaker
    Satellite,
}

impl HierarchicalItem {
    /// Returns the type of this hierarchical item
    pub fn item_type(&self) -> ItemType {
        match self {
            HierarchicalItem::Group { .. } => ItemType::Group,
            HierarchicalItem::Speaker { .. } => ItemType::Speaker,
            HierarchicalItem::Satellite { .. } => ItemType::Satellite,
        }
    }

    /// Creates a HierarchicalItem::Group from a Group
    pub fn from_group(group: &Group) -> Self {
        HierarchicalItem::Group {
            name: group.name.clone(),
            member_count: group.speakers.len(),
        }
    }

    /// Creates a HierarchicalItem::Speaker from speaker name and group info
    pub fn from_speaker(speaker_name: &str, group_name: &str, is_coordinator: bool) -> Self {
        HierarchicalItem::Speaker {
            name: speaker_name.to_string(),
            group_name: group_name.to_string(),
            is_coordinator,
        }
    }

    /// Creates a HierarchicalItem::Satellite from satellite info
    pub fn from_satellite(satellite_name: &str, parent_speaker_name: &str, group_name: &str) -> Self {
        HierarchicalItem::Satellite {
            name: satellite_name.to_string(),
            parent_speaker_name: parent_speaker_name.to_string(),
            group_name: group_name.to_string(),
        }
    }
}

/// Widget for displaying a hierarchical list of Sonos topology with zone groups and speakers
#[derive(Clone)]
pub struct TopologyList {
    /// The underlying selectable list widget for navigation and rendering
    list: SelectableList,
    /// The hierarchical items that can be selected
    items: Vec<HierarchicalItem>,
}

impl TopologyList {
    /// Creates a new TopologyList widget from a simplified Topology reference
    /// 
    /// # Arguments
    /// * `topology` - Reference to the simplified Sonos topology data
    /// 
    /// # Returns
    /// A new TopologyList widget ready for display and navigation
    pub fn new(topology: &Topology) -> Self {
        let (items, display_strings) = TopologyFlattener::flatten(topology);
        let list = SelectableList::new("Topology", display_strings);
        
        Self {
            list,
            items,
        }
    }

    /// Creates a new TopologyList widget from a full SonosTopology reference
    /// This version supports satellites and provides enhanced visual hierarchy
    /// 
    /// # Arguments
    /// * `topology` - Reference to the full Sonos topology data with satellite support
    /// 
    /// # Returns
    /// A new TopologyList widget ready for display and navigation
    pub fn from_sonos_topology(topology: &SonosTopology) -> Self {
        let (items, display_strings) = TopologyFlattener::flatten_sonos_topology(topology);
        let list = SelectableList::new("Topology", display_strings);
        
        Self {
            list,
            items,
        }
    }

    /// Returns the currently selected hierarchical item
    /// 
    /// This method provides safe access to the currently selected item with proper
    /// error handling for edge cases like empty lists or invalid selection indices.
    /// 
    /// # Returns
    /// * `Some(&HierarchicalItem)` - Reference to the selected item if valid selection exists
    /// * `None` - If no item is selected, list is empty, or selection index is invalid
    /// 
    /// # Examples
    /// ```
    /// let topology_list = TopologyList::new(&topology);
    /// match topology_list.selected_item() {
    ///     Some(HierarchicalItem::Group { name, .. }) => {
    ///         println!("Selected group: {}", name);
    ///     }
    ///     Some(HierarchicalItem::Speaker { name, .. }) => {
    ///         println!("Selected speaker: {}", name);
    ///     }
    ///     None => println!("No item selected"),
    /// }
    /// ```
    pub fn selected_item(&self) -> Option<&HierarchicalItem> {
        // Handle empty list case
        if self.items.is_empty() {
            return None;
        }

        // Get selected index and validate it's within bounds
        match self.list.selected() {
            Some(index) => {
                // Double-check bounds to handle any potential race conditions
                // or inconsistencies between the list widget and our items
                if index < self.items.len() {
                    self.items.get(index)
                } else {
                    // Log error in debug builds for development
                    #[cfg(debug_assertions)]
                    eprintln!("Warning: Selection index {} out of bounds for {} items", index, self.items.len());
                    None
                }
            }
            None => None,
        }
    }

    /// Returns the type of the currently selected item
    /// 
    /// This is a convenience method that combines selection retrieval with type
    /// identification, providing safe access to the item type without needing
    /// to pattern match on the full HierarchicalItem enum.
    /// 
    /// # Returns
    /// * `Some(ItemType)` - The type of the selected item (Group or Speaker)
    /// * `None` - If no item is selected or selection is invalid
    /// 
    /// # Examples
    /// ```
    /// match topology_list.selected_item_type() {
    ///     Some(ItemType::Group) => handle_group_selection(),
    ///     Some(ItemType::Speaker) => handle_speaker_selection(),
    ///     None => handle_no_selection(),
    /// }
    /// ```
    pub fn selected_item_type(&self) -> Option<ItemType> {
        self.selected_item().map(|item| item.item_type())
    }

    /// Renders the hierarchical topology list to the terminal
    /// 
    /// # Arguments
    /// * `frame` - The ratatui Frame to render to
    /// * `area` - The rectangular area to render within
    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        self.list.draw(frame, area);
    }

    /// Moves selection to the next item in the hierarchical list
    /// Wraps to the first item when reaching the end
    /// Does nothing if the list is empty
    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.list.next();
        }
    }

    /// Moves selection to the previous item in the hierarchical list
    /// Wraps to the last item when reaching the beginning
    /// Does nothing if the list is empty
    pub fn previous(&mut self) {
        if !self.items.is_empty() {
            self.list.previous();
        }
    }

    /// Returns the index of the currently selected item
    /// 
    /// # Returns
    /// Some index of the selected item, or None if no selection
    pub fn selected(&self) -> Option<usize> {
        if self.items.is_empty() {
            None
        } else {
            self.list.selected()
        }
    }

    /// Returns the total number of items in the hierarchical list
    /// 
    /// # Returns
    /// The number of selectable items (groups and speakers)
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true if the topology list is empty
    /// 
    /// # Returns
    /// true if there are no items to display, false otherwise
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Topology, Group, SonosTopology, ZoneGroup, ZoneGroupMember, Satellite};

    fn create_test_group() -> Group {
        Group {
            name: "Living Room".to_string(),
            speakers: vec!["Living Room".to_string()],
        }
    }

    fn create_test_topology() -> Topology {
        Topology {
            groups: vec![create_test_group()],
        }
    }

    fn create_multi_group_topology() -> Topology {
        Topology {
            groups: vec![
                Group {
                    name: "Living Room".to_string(),
                    speakers: vec!["Living Room".to_string(), "Kitchen".to_string()],
                },
                Group {
                    name: "Bedroom".to_string(),
                    speakers: vec!["Bedroom".to_string()],
                },
            ],
        }
    }

    fn create_empty_topology() -> Topology {
        Topology {
            groups: vec![],
        }
    }

    #[test]
    fn test_hierarchical_item_group_creation() {
        let group = create_test_group();
        let item = HierarchicalItem::from_group(&group);

        match item {
            HierarchicalItem::Group { name, member_count } => {
                assert_eq!(name, "Living Room");
                assert_eq!(member_count, 1);
            }
            _ => panic!("Expected Group variant"),
        }
    }

    #[test]
    fn test_hierarchical_item_speaker_creation() {
        let item = HierarchicalItem::from_speaker("Living Room", "Living Room", true);

        match item {
            HierarchicalItem::Speaker { name, group_name, is_coordinator } => {
                assert_eq!(name, "Living Room");
                assert_eq!(group_name, "Living Room");
                assert!(is_coordinator);
            }
            _ => panic!("Expected Speaker variant"),
        }
    }

    #[test]
    fn test_hierarchical_item_satellite_creation() {
        let item = HierarchicalItem::from_satellite("Surround Left", "Living Room", "Living Room");

        match item {
            HierarchicalItem::Satellite { name, parent_speaker_name, group_name } => {
                assert_eq!(name, "Surround Left");
                assert_eq!(parent_speaker_name, "Living Room");
                assert_eq!(group_name, "Living Room");
            }
            _ => panic!("Expected Satellite variant"),
        }
    }

    #[test]
    fn test_item_type_identification() {
        let group = create_test_group();
        let group_item = HierarchicalItem::from_group(&group);
        assert_eq!(group_item.item_type(), ItemType::Group);

        let speaker_item = HierarchicalItem::from_speaker("Living Room", "Living Room", false);
        assert_eq!(speaker_item.item_type(), ItemType::Speaker);

        let satellite_item = HierarchicalItem::from_satellite("Surround Left", "Living Room", "Living Room");
        assert_eq!(satellite_item.item_type(), ItemType::Satellite);
    }

    #[test]
    fn test_topology_flattener_single_group() {
        let topology = create_test_topology();
        let (items, display_strings) = TopologyFlattener::flatten(&topology);

        // Should have: 1 group + 1 speaker = 2 items
        assert_eq!(items.len(), 2);
        assert_eq!(display_strings.len(), 2);

        // Check the group
        match &items[0] {
            HierarchicalItem::Group { name, member_count } => {
                assert_eq!(name, "Living Room");
                assert_eq!(*member_count, 1);
            }
            _ => panic!("Expected Group at index 0"),
        }
        assert_eq!(display_strings[0], "Group: Living Room (1 speaker)");

        // Check the speaker
        match &items[1] {
            HierarchicalItem::Speaker { name, group_name, is_coordinator } => {
                assert_eq!(name, "Living Room");
                assert_eq!(group_name, "Living Room");
                assert!(*is_coordinator);
            }
            _ => panic!("Expected Speaker at index 1"),
        }
        assert_eq!(display_strings[1], "  Speaker: Living Room");
    }

    #[test]
    fn test_topology_flattener_multi_group_topology() {
        let topology = create_multi_group_topology();
        let (items, display_strings) = TopologyFlattener::flatten(&topology);

        // Should have: 2 groups + 3 speakers = 5 items
        assert_eq!(items.len(), 5);
        assert_eq!(display_strings.len(), 5);

        // First group
        assert!(matches!(items[0], HierarchicalItem::Group { .. }));
        assert_eq!(display_strings[0], "Group: Living Room (2 speakers)");

        // First group's coordinator (Living Room)
        match &items[1] {
            HierarchicalItem::Speaker { name, group_name, is_coordinator } => {
                assert_eq!(name, "Living Room");
                assert_eq!(group_name, "Living Room");
                assert!(*is_coordinator);
            }
            _ => panic!("Expected coordinator speaker at index 1"),
        }

        // First group's second member (Kitchen)
        match &items[2] {
            HierarchicalItem::Speaker { name, group_name, is_coordinator } => {
                assert_eq!(name, "Kitchen");
                assert_eq!(group_name, "Living Room");
                assert!(!*is_coordinator);
            }
            _ => panic!("Expected non-coordinator speaker at index 2"),
        }

        // Second group
        assert!(matches!(items[3], HierarchicalItem::Group { .. }));
        assert_eq!(display_strings[3], "Group: Bedroom (1 speaker)");

        // Second group's speaker
        match &items[4] {
            HierarchicalItem::Speaker { name, group_name, is_coordinator } => {
                assert_eq!(name, "Bedroom");
                assert_eq!(group_name, "Bedroom");
                assert!(*is_coordinator);
            }
            _ => panic!("Expected coordinator speaker at index 4"),
        }
    }

    #[test]
    fn test_topology_flattener_empty_topology() {
        let topology = create_empty_topology();
        let (items, display_strings) = TopologyFlattener::flatten(&topology);

        assert_eq!(items.len(), 0);
        assert_eq!(display_strings.len(), 0);
    }

    #[test]
    fn test_topology_flattener_edge_cases() {
        // Test group with empty speakers list
        let topology_with_empty_group = Topology {
            groups: vec![
                Group {
                    name: "Valid Group".to_string(),
                    speakers: vec!["Speaker 1".to_string()],
                },
                Group {
                    name: "Empty Group".to_string(),
                    speakers: vec![], // This should be skipped
                },
            ],
        };

        let (items, display_strings) = TopologyFlattener::flatten(&topology_with_empty_group);
        
        // Should only have items from the valid group (1 group + 1 speaker = 2 items)
        assert_eq!(items.len(), 2);
        assert_eq!(display_strings.len(), 2);
        
        // Verify the valid group is processed correctly
        match &items[0] {
            HierarchicalItem::Group { name, member_count } => {
                assert_eq!(name, "Valid Group");
                assert_eq!(*member_count, 1);
            }
            _ => panic!("Expected Group at index 0"),
        }
    }

    #[test]
    fn test_topology_list_creation_single_group() {
        let topology = create_test_topology();
        let topology_list = TopologyList::new(&topology);

        assert_eq!(topology_list.len(), 2); // 1 group + 1 speaker
        assert!(!topology_list.is_empty());
        assert_eq!(topology_list.selected(), Some(0)); // Should start with first item selected
    }

    #[test]
    fn test_topology_list_creation_multi_group() {
        let topology = create_multi_group_topology();
        let topology_list = TopologyList::new(&topology);

        assert_eq!(topology_list.len(), 5); // 2 groups + 3 speakers
        assert!(!topology_list.is_empty());
        assert_eq!(topology_list.selected(), Some(0));
    }

    #[test]
    fn test_topology_list_creation_empty_topology() {
        let topology = create_empty_topology();
        let topology_list = TopologyList::new(&topology);

        assert_eq!(topology_list.len(), 0);
        assert!(topology_list.is_empty());
        assert_eq!(topology_list.selected(), None);
    }

    #[test]
    fn test_topology_list_navigation_single_group() {
        let topology = create_test_topology();
        let mut topology_list = TopologyList::new(&topology);

        // Start at index 0 (group)
        assert_eq!(topology_list.selected(), Some(0));
        
        // Move to next item (speaker)
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(1));
        
        // Move to next item (should wrap to first)
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(0));
        
        // Move to previous item (should wrap to last)
        topology_list.previous();
        assert_eq!(topology_list.selected(), Some(1));
        
        // Move to previous item (group)
        topology_list.previous();
        assert_eq!(topology_list.selected(), Some(0));
    }

    #[test]
    fn test_topology_list_navigation_multi_group() {
        let topology = create_multi_group_topology();
        let mut topology_list = TopologyList::new(&topology);

        // Test navigation through all items: 0 -> 1 -> 2 -> 3 -> 4 -> 0 (wrap)
        assert_eq!(topology_list.selected(), Some(0));
        
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(1));
        
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(2));
        
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(3));
        
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(4));
        
        topology_list.next(); // Should wrap to 0
        assert_eq!(topology_list.selected(), Some(0));

        // Test reverse navigation
        topology_list.previous(); // Should wrap to index 4
        assert_eq!(topology_list.selected(), Some(4));
        
        topology_list.previous(); // To index 3
        assert_eq!(topology_list.selected(), Some(3));
    }

    #[test]
    fn test_topology_list_navigation_empty_list() {
        let topology = create_empty_topology();
        let mut topology_list = TopologyList::new(&topology);

        // Navigation should do nothing on empty list
        topology_list.next();
        assert_eq!(topology_list.selected(), None);
        
        topology_list.previous();
        assert_eq!(topology_list.selected(), None);
    }

    #[test]
    fn test_topology_list_selected_item_group() {
        let topology = create_test_topology();
        let topology_list = TopologyList::new(&topology);

        // Should start with group selected
        let selected = topology_list.selected_item();
        assert!(selected.is_some());
        
        match selected.unwrap() {
            HierarchicalItem::Group { name, member_count } => {
                assert_eq!(name, "Living Room");
                assert_eq!(*member_count, 1);
            }
            _ => panic!("Expected Group to be selected"),
        }
    }

    #[test]
    fn test_topology_list_selected_item_speaker() {
        let topology = create_test_topology();
        let mut topology_list = TopologyList::new(&topology);

        // Move to speaker
        topology_list.next();
        
        let selected = topology_list.selected_item();
        assert!(selected.is_some());
        
        match selected.unwrap() {
            HierarchicalItem::Speaker { name, group_name, is_coordinator } => {
                assert_eq!(name, "Living Room");
                assert_eq!(group_name, "Living Room");
                assert!(*is_coordinator);
            }
            _ => panic!("Expected Speaker to be selected"),
        }
    }

    #[test]
    fn test_topology_list_selected_item_type() {
        let topology = create_test_topology();
        let mut topology_list = TopologyList::new(&topology);

        // Should start with group selected
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Group));
        
        // Move to speaker
        topology_list.next();
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Speaker));
    }

    #[test]
    fn test_topology_list_selected_item_empty_list() {
        let topology = create_empty_topology();
        let topology_list = TopologyList::new(&topology);

        assert!(topology_list.selected_item().is_none());
        assert!(topology_list.selected_item_type().is_none());
    }

    #[test]
    fn test_topology_list_edge_case_single_speaker_group() {
        let topology = Topology {
            groups: vec![Group {
                name: "Bedroom".to_string(),
                speakers: vec!["Bedroom".to_string()],
            }],
        };
        
        let topology_list = TopologyList::new(&topology);
        assert_eq!(topology_list.len(), 2); // 1 group + 1 speaker
        
        // Check group
        let group_item = topology_list.selected_item().unwrap();
        match group_item {
            HierarchicalItem::Group { name, member_count } => {
                assert_eq!(name, "Bedroom");
                assert_eq!(*member_count, 1);
            }
            _ => panic!("Expected Group"),
        }
    }

    #[test]
    fn test_topology_list_edge_case_group_with_empty_name() {
        let topology = Topology {
            groups: vec![Group {
                name: "".to_string(), // Empty name
                speakers: vec!["Speaker 1".to_string()],
            }],
        };
        
        let topology_list = TopologyList::new(&topology);
        assert_eq!(topology_list.len(), 2);
        
        // Should use fallback name
        let group_item = topology_list.selected_item().unwrap();
        match group_item {
            HierarchicalItem::Group { name, .. } => {
                assert_eq!(name, "Unknown Group");
            }
            _ => panic!("Expected Group"),
        }
    }

    #[test]
    fn test_topology_list_edge_case_speaker_with_empty_name() {
        let topology = Topology {
            groups: vec![Group {
                name: "Living Room".to_string(),
                speakers: vec!["".to_string()], // Empty speaker name
            }],
        };
        
        let mut topology_list = TopologyList::new(&topology);
        topology_list.next(); // Move to speaker
        
        let speaker_item = topology_list.selected_item().unwrap();
        match speaker_item {
            HierarchicalItem::Speaker { name, .. } => {
                assert_eq!(name, "Unknown Speaker");
            }
            _ => panic!("Expected Speaker"),
        }
    }

    // Tests for SonosTopology (full topology with satellites)
    
    fn create_test_satellite() -> Satellite {
        Satellite {
            uuid: "RINCON_SAT123".to_string(),
            location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
            zone_name: "Surround Left".to_string(),
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

    fn create_test_sonos_topology() -> SonosTopology {
        SonosTopology {
            zone_groups: vec![create_test_zone_group()],
            vanished_devices: None,
        }
    }

    fn create_multi_group_sonos_topology() -> SonosTopology {
        SonosTopology {
            zone_groups: vec![
                create_test_zone_group(),
                ZoneGroup {
                    coordinator: "RINCON_789".to_string(),
                    id: "RINCON_789:987".to_string(),
                    members: vec![
                        ZoneGroupMember {
                            uuid: "RINCON_789".to_string(),
                            location: "http://192.168.1.102:1400/xml/device_description.xml".to_string(),
                            zone_name: "Kitchen".to_string(),
                            software_version: "56.0-76060".to_string(),
                            configuration: "1".to_string(),
                            icon: "x-rincon-roomicon:kitchen".to_string(),
                            satellites: vec![],
                        },
                        ZoneGroupMember {
                            uuid: "RINCON_ABC".to_string(),
                            location: "http://192.168.1.103:1400/xml/device_description.xml".to_string(),
                            zone_name: "Dining Room".to_string(),
                            software_version: "56.0-76060".to_string(),
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

    fn create_empty_sonos_topology() -> SonosTopology {
        SonosTopology {
            zone_groups: vec![],
            vanished_devices: None,
        }
    }

    #[test]
    fn test_topology_list_from_sonos_topology_single_group_with_satellite() {
        let topology = create_test_sonos_topology();
        let topology_list = TopologyList::from_sonos_topology(&topology);

        // Should have: 1 group + 1 speaker + 1 satellite = 3 items
        assert_eq!(topology_list.len(), 3);
        assert!(!topology_list.is_empty());
        
        // Check group
        let group_item = &topology_list.items[0];
        match group_item {
            HierarchicalItem::Group { name, member_count } => {
                assert_eq!(name, "Living Room");
                assert_eq!(*member_count, 1);
            }
            _ => panic!("Expected Group at index 0"),
        }
        
        // Check speaker
        let speaker_item = &topology_list.items[1];
        match speaker_item {
            HierarchicalItem::Speaker { name, group_name, is_coordinator } => {
                assert_eq!(name, "Living Room");
                assert_eq!(group_name, "Living Room");
                assert!(*is_coordinator);
            }
            _ => panic!("Expected Speaker at index 1"),
        }
        
        // Check satellite
        let satellite_item = &topology_list.items[2];
        match satellite_item {
            HierarchicalItem::Satellite { name, parent_speaker_name, group_name } => {
                assert_eq!(name, "Surround Left");
                assert_eq!(parent_speaker_name, "Living Room");
                assert_eq!(group_name, "Living Room");
            }
            _ => panic!("Expected Satellite at index 2"),
        }
    }

    #[test]
    fn test_topology_list_from_sonos_topology_multi_group() {
        let topology = create_multi_group_sonos_topology();
        let topology_list = TopologyList::from_sonos_topology(&topology);

        // Should have: 2 groups + 3 speakers + 1 satellite = 6 items
        assert_eq!(topology_list.len(), 6);
        
        // First group (Living Room with satellite)
        assert!(matches!(topology_list.items[0], HierarchicalItem::Group { .. }));
        assert!(matches!(topology_list.items[1], HierarchicalItem::Speaker { .. }));
        assert!(matches!(topology_list.items[2], HierarchicalItem::Satellite { .. }));
        
        // Second group (Kitchen + Dining Room, no satellites)
        assert!(matches!(topology_list.items[3], HierarchicalItem::Group { .. }));
        assert!(matches!(topology_list.items[4], HierarchicalItem::Speaker { .. })); // Kitchen (coordinator)
        assert!(matches!(topology_list.items[5], HierarchicalItem::Speaker { .. })); // Dining Room
    }

    #[test]
    fn test_topology_list_from_sonos_topology_empty() {
        let topology = create_empty_sonos_topology();
        let topology_list = TopologyList::from_sonos_topology(&topology);

        assert_eq!(topology_list.len(), 0);
        assert!(topology_list.is_empty());
        assert!(topology_list.selected_item().is_none());
    }

    #[test]
    fn test_topology_list_navigation_with_satellites() {
        let topology = create_test_sonos_topology();
        let mut topology_list = TopologyList::from_sonos_topology(&topology);

        // Navigate through all items: Group -> Speaker -> Satellite -> Group (wrap)
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Group));
        
        topology_list.next();
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Speaker));
        
        topology_list.next();
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Satellite));
        
        topology_list.next(); // Should wrap to group
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Group));
        
        // Test reverse navigation
        topology_list.previous(); // Should wrap to satellite
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Satellite));
    }

    #[test]
    fn test_topology_list_selected_satellite() {
        let topology = create_test_sonos_topology();
        let mut topology_list = TopologyList::from_sonos_topology(&topology);

        // Navigate to satellite
        topology_list.next(); // Speaker
        topology_list.next(); // Satellite
        
        let selected = topology_list.selected_item();
        assert!(selected.is_some());
        
        match selected.unwrap() {
            HierarchicalItem::Satellite { name, parent_speaker_name, group_name } => {
                assert_eq!(name, "Surround Left");
                assert_eq!(parent_speaker_name, "Living Room");
                assert_eq!(group_name, "Living Room");
            }
            _ => panic!("Expected Satellite to be selected"),
        }
    }

    #[test]
    fn test_topology_list_edge_case_missing_coordinator() {
        let topology = SonosTopology {
            zone_groups: vec![ZoneGroup {
                coordinator: "RINCON_MISSING".to_string(), // Coordinator not in members
                id: "RINCON_123:456".to_string(),
                members: vec![ZoneGroupMember {
                    uuid: "RINCON_ACTUAL".to_string(),
                    location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
                    zone_name: "Living Room".to_string(),
                    software_version: "56.0-76060".to_string(),
                    configuration: "1".to_string(),
                    icon: "x-rincon-roomicon:living".to_string(),
                    satellites: vec![],
                }],
            }],
            vanished_devices: None,
        };
        
        let topology_list = TopologyList::from_sonos_topology(&topology);
        assert_eq!(topology_list.len(), 2); // Should still create group and speaker
        
        // Should use first member's name as fallback
        let group_item = &topology_list.items[0];
        match group_item {
            HierarchicalItem::Group { name, .. } => {
                assert_eq!(name, "Living Room");
            }
            _ => panic!("Expected Group"),
        }
    }

    #[test]
    fn test_topology_list_edge_case_empty_zone_names() {
        let topology = SonosTopology {
            zone_groups: vec![ZoneGroup {
                coordinator: "RINCON_123".to_string(),
                id: "RINCON_123:456".to_string(),
                members: vec![
                    ZoneGroupMember {
                        uuid: "RINCON_123".to_string(),
                        location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
                        zone_name: "".to_string(), // Empty zone name
                        software_version: "56.0-76060".to_string(),
                        configuration: "1".to_string(),
                        icon: "x-rincon-roomicon:living".to_string(),
                        satellites: vec![Satellite {
                            uuid: "RINCON_SAT".to_string(),
                            location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
                            zone_name: "".to_string(), // Empty satellite name
                            software_version: "56.0-76060".to_string(),
                        }],
                    },
                ],
            }],
            vanished_devices: None,
        };
        
        let topology_list = TopologyList::from_sonos_topology(&topology);
        assert_eq!(topology_list.len(), 3);
        
        // Check that fallback names are used
        let speaker_item = &topology_list.items[1];
        match speaker_item {
            HierarchicalItem::Speaker { name, .. } => {
                assert!(name.starts_with("Unknown Speaker"));
            }
            _ => panic!("Expected Speaker"),
        }
        
        let satellite_item = &topology_list.items[2];
        match satellite_item {
            HierarchicalItem::Satellite { name, .. } => {
                assert!(name.starts_with("Unknown Satellite"));
            }
            _ => panic!("Expected Satellite"),
        }
    }

    #[test]
    fn test_topology_list_coordinator_ordering() {
        let topology = SonosTopology {
            zone_groups: vec![ZoneGroup {
                coordinator: "RINCON_COORD".to_string(),
                id: "RINCON_GROUP:123".to_string(),
                members: vec![
                    ZoneGroupMember {
                        uuid: "RINCON_ALPHA".to_string(),
                        location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
                        zone_name: "Alpha Speaker".to_string(),
                        software_version: "56.0-76060".to_string(),
                        configuration: "1".to_string(),
                        icon: "x-rincon-roomicon:living".to_string(),
                        satellites: vec![],
                    },
                    ZoneGroupMember {
                        uuid: "RINCON_COORD".to_string(), // This is the coordinator
                        location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
                        zone_name: "Coordinator Speaker".to_string(),
                        software_version: "56.0-76060".to_string(),
                        configuration: "1".to_string(),
                        icon: "x-rincon-roomicon:living".to_string(),
                        satellites: vec![],
                    },
                    ZoneGroupMember {
                        uuid: "RINCON_ZULU".to_string(),
                        location: "http://192.168.1.102:1400/xml/device_description.xml".to_string(),
                        zone_name: "Zulu Speaker".to_string(),
                        software_version: "56.0-76060".to_string(),
                        configuration: "1".to_string(),
                        icon: "x-rincon-roomicon:living".to_string(),
                        satellites: vec![],
                    },
                ],
            }],
            vanished_devices: None,
        };
        
        let topology_list = TopologyList::from_sonos_topology(&topology);
        assert_eq!(topology_list.len(), 4); // 1 group + 3 speakers
        
        // Coordinator should be first speaker (index 1)
        let first_speaker = &topology_list.items[1];
        match first_speaker {
            HierarchicalItem::Speaker { name, is_coordinator, .. } => {
                assert_eq!(name, "Coordinator Speaker");
                assert!(*is_coordinator);
            }
            _ => panic!("Expected coordinator speaker at index 1"),
        }
        
        // Other speakers should be in alphabetical order
        let second_speaker = &topology_list.items[2];
        match second_speaker {
            HierarchicalItem::Speaker { name, is_coordinator, .. } => {
                assert_eq!(name, "Alpha Speaker");
                assert!(!*is_coordinator);
            }
            _ => panic!("Expected Alpha speaker at index 2"),
        }
        
        let third_speaker = &topology_list.items[3];
        match third_speaker {
            HierarchicalItem::Speaker { name, is_coordinator, .. } => {
                assert_eq!(name, "Zulu Speaker");
                assert!(!*is_coordinator);
            }
            _ => panic!("Expected Zulu speaker at index 3"),
        }
    }

    #[test]
    fn test_navigation_wrapping_forward() {
        let topology = create_multi_group_topology();
        let mut topology_list = TopologyList::new(&topology);
        
        // Should start at index 0
        assert_eq!(topology_list.selected(), Some(0));
        
        // Navigate through all items
        let total_items = topology_list.len();
        for i in 1..total_items {
            topology_list.next();
            assert_eq!(topology_list.selected(), Some(i));
        }
        
        // Next should wrap to first item
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(0));
        
        // Verify we can continue wrapping
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(1));
    }

    #[test]
    fn test_navigation_wrapping_backward() {
        let topology = create_multi_group_topology();
        let mut topology_list = TopologyList::new(&topology);
        
        // Should start at index 0
        assert_eq!(topology_list.selected(), Some(0));
        
        // Previous should wrap to last item
        let last_index = topology_list.len() - 1;
        topology_list.previous();
        assert_eq!(topology_list.selected(), Some(last_index));
        
        // Navigate backward through all items
        for i in (0..last_index).rev() {
            topology_list.previous();
            assert_eq!(topology_list.selected(), Some(i));
        }
        
        // Previous should wrap to last item again
        topology_list.previous();
        assert_eq!(topology_list.selected(), Some(last_index));
    }

    #[test]
    fn test_navigation_wrapping_single_item() {
        let topology = create_test_topology(); // Has only 2 items (1 group + 1 speaker)
        let mut topology_list = TopologyList::new(&topology);
        
        assert_eq!(topology_list.len(), 2);
        assert_eq!(topology_list.selected(), Some(0));
        
        // Navigate forward through both items
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(1));
        
        // Should wrap to first
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(0));
        
        // Navigate backward should wrap to last
        topology_list.previous();
        assert_eq!(topology_list.selected(), Some(1));
        
        // Navigate backward again
        topology_list.previous();
        assert_eq!(topology_list.selected(), Some(0));
    }

    #[test]
    fn test_navigation_empty_topology() {
        let topology = create_empty_topology();
        let mut topology_list = TopologyList::new(&topology);
        
        // Empty topology should have no items
        assert_eq!(topology_list.len(), 0);
        assert_eq!(topology_list.selected(), None);
        
        // Navigation should not panic on empty list
        topology_list.next();
        assert_eq!(topology_list.selected(), None);
        
        topology_list.previous();
        assert_eq!(topology_list.selected(), None);
        
        // Selected item should be None
        assert!(topology_list.selected_item().is_none());
        assert!(topology_list.selected_item_type().is_none());
    }

    #[test]
    fn test_navigation_selection_consistency() {
        let topology = create_multi_group_topology();
        let mut topology_list = TopologyList::new(&topology);
        
        let total_items = topology_list.len();
        
        // Test that selection state remains consistent during navigation
        for _ in 0..total_items * 2 {
            let selected_index = topology_list.selected().unwrap();
            let selected_item = topology_list.selected_item().unwrap();
            let selected_type = topology_list.selected_item_type().unwrap();
            
            // Verify consistency between index and item
            assert!(selected_index < total_items);
            assert_eq!(selected_item.item_type(), selected_type);
            
            topology_list.next();
        }
        
        // Test backward navigation consistency
        for _ in 0..total_items * 2 {
            let selected_index = topology_list.selected().unwrap();
            let selected_item = topology_list.selected_item().unwrap();
            let selected_type = topology_list.selected_item_type().unwrap();
            
            // Verify consistency between index and item
            assert!(selected_index < total_items);
            assert_eq!(selected_item.item_type(), selected_type);
            
            topology_list.previous();
        }
    }

    #[test]
    fn test_navigation_through_complex_hierarchy() {
        // Create a complex topology with multiple groups and different item types
        let topology = Topology {
            groups: vec![
                Group {
                    name: "Living Room".to_string(),
                    speakers: vec!["Living Room".to_string(), "Kitchen".to_string(), "Dining Room".to_string()],
                },
                Group {
                    name: "Bedroom".to_string(),
                    speakers: vec!["Bedroom".to_string()],
                },
                Group {
                    name: "Office".to_string(),
                    speakers: vec!["Office".to_string(), "Study".to_string()],
                },
            ],
        };
        
        let mut topology_list = TopologyList::new(&topology);
        let total_items = topology_list.len();
        
        // Should have: 3 groups + 6 speakers = 9 items
        assert_eq!(total_items, 9);
        
        // Test navigation through all hierarchy levels
        let mut group_count = 0;
        let mut speaker_count = 0;
        
        for i in 0..total_items {
            let item_type = topology_list.selected_item_type().unwrap();
            match item_type {
                ItemType::Group => group_count += 1,
                ItemType::Speaker => speaker_count += 1,
                ItemType::Satellite => {} // Not present in simplified topology
            }
            
            // Verify we can navigate to next item
            let current_index = topology_list.selected().unwrap();
            assert_eq!(current_index, i);
            
            topology_list.next();
        }
        
        // Should have found 3 groups and 6 speakers
        assert_eq!(group_count, 3);
        assert_eq!(speaker_count, 6);
        
        // After navigating through all items, should wrap to first
        assert_eq!(topology_list.selected(), Some(0));
    }

    #[test]
    fn test_navigation_wrapping_bounds_checking() {
        let topology = create_test_topology();
        let mut topology_list = TopologyList::new(&topology);
        
        let total_items = topology_list.len();
        
        // Perform many navigation operations to test bounds checking
        for _ in 0..100 {
            topology_list.next();
            let selected = topology_list.selected().unwrap();
            assert!(selected < total_items, "Selection index {} out of bounds for {} items", selected, total_items);
        }
        
        for _ in 0..100 {
            topology_list.previous();
            let selected = topology_list.selected().unwrap();
            assert!(selected < total_items, "Selection index {} out of bounds for {} items", selected, total_items);
        }
    }

    #[test]
    fn test_topology_flattener_malformed_data() {
        // Test with empty group names and speaker names
        let malformed_topology = Topology {
            groups: vec![
                Group {
                    name: "".to_string(), // Empty group name
                    speakers: vec!["Valid Speaker".to_string(), "".to_string()], // One empty speaker name
                },
                Group {
                    name: "   ".to_string(), // Whitespace-only group name
                    speakers: vec!["Another Speaker".to_string()],
                },
            ],
        };

        let (items, _display_strings) = TopologyFlattener::flatten(&malformed_topology);
        
        // Should have 5 items: 2 groups + 3 speakers (empty speaker name gets placeholder)
        // First group: 1 group + 2 speakers (including placeholder)
        // Second group: 1 group + 1 speaker
        assert_eq!(items.len(), 5);
        
        // Check first group with empty name gets placeholder
        match &items[0] {
            HierarchicalItem::Group { name, .. } => {
                assert_eq!(name, "Unknown Group");
            }
            _ => panic!("Expected Group at index 0"),
        }
        
        // Check that empty speaker name gets placeholder
        let has_unknown_speaker = items.iter().any(|item| {
            matches!(item, HierarchicalItem::Speaker { name, .. } if name == "Unknown Speaker")
        });
        assert!(has_unknown_speaker, "Should have placeholder for empty speaker name");
    }

    #[test]
    fn test_topology_flattener_missing_coordinator() {
        // Test group where coordinator name doesn't match any speaker
        let topology_missing_coordinator = Topology {
            groups: vec![
                Group {
                    name: "Missing Coordinator".to_string(),
                    speakers: vec!["Speaker A".to_string(), "Speaker B".to_string()],
                },
            ],
        };

        let (items, _display_strings) = TopologyFlattener::flatten(&topology_missing_coordinator);
        
        // Should have 3 items: 1 group + 2 speakers
        assert_eq!(items.len(), 3);
        
        // Check that first speaker is marked as coordinator (fallback behavior)
        match &items[1] {
            HierarchicalItem::Speaker { name, is_coordinator, .. } => {
                assert_eq!(name, "Speaker A");
                assert!(*is_coordinator, "First speaker should be marked as coordinator when original coordinator is missing");
            }
            _ => panic!("Expected Speaker at index 1"),
        }
    }

    #[test]
    fn test_sonos_topology_flattener_edge_cases() {
        use crate::types::{SonosTopology, ZoneGroup};

        // Test empty SonosTopology
        let empty_sonos_topology = SonosTopology {
            zone_groups: vec![],
            vanished_devices: None,
        };

        let (items, display_strings) = TopologyFlattener::flatten_sonos_topology(&empty_sonos_topology);
        assert_eq!(items.len(), 0);
        assert_eq!(display_strings.len(), 0);

        // Test zone group with empty members
        let topology_with_empty_group = SonosTopology {
            zone_groups: vec![
                ZoneGroup {
                    coordinator: "RINCON_123".to_string(),
                    id: "RINCON_123:123".to_string(),
                    members: vec![], // Empty members - should be skipped
                },
            ],
            vanished_devices: None,
        };

        let (items, _display_strings) = TopologyFlattener::flatten_sonos_topology(&topology_with_empty_group);
        assert_eq!(items.len(), 0);
        assert_eq!(display_strings.len(), 0);
    }

    #[test]
    fn test_sonos_topology_missing_coordinator() {
        use crate::types::{SonosTopology, ZoneGroup, ZoneGroupMember};

        // Test zone group where coordinator UUID doesn't match any member
        let member = ZoneGroupMember {
            uuid: "RINCON_456".to_string(),
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            zone_name: "Living Room".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![],
        };

        let zone_group = ZoneGroup {
            coordinator: "RINCON_MISSING".to_string(), // This UUID doesn't match any member
            id: "RINCON_MISSING:123".to_string(),
            members: vec![member],
        };

        let topology = SonosTopology {
            zone_groups: vec![zone_group],
            vanished_devices: None,
        };

        let (items, _display_strings) = TopologyFlattener::flatten_sonos_topology(&topology);
        
        // Should have 2 items: 1 group + 1 speaker
        assert_eq!(items.len(), 2);
        
        // Group should use first member's name as fallback
        match &items[0] {
            HierarchicalItem::Group { name, .. } => {
                assert_eq!(name, "Living Room");
            }
            _ => panic!("Expected Group at index 0"),
        }
        
        // Speaker should not be marked as coordinator since UUID doesn't match
        match &items[1] {
            HierarchicalItem::Speaker { name, is_coordinator, .. } => {
                assert_eq!(name, "Living Room");
                assert!(!*is_coordinator, "Speaker should not be marked as coordinator when UUID doesn't match");
            }
            _ => panic!("Expected Speaker at index 1"),
        }
    }

    #[test]
    fn test_sonos_topology_empty_zone_names() {
        use crate::types::{SonosTopology, ZoneGroup, ZoneGroupMember, Satellite};

        let satellite_with_empty_name = Satellite {
            uuid: "RINCON_SAT123".to_string(),
            location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
            zone_name: "".to_string(), // Empty zone name
            software_version: "56.0-76060".to_string(),
        };

        let member_with_empty_name = ZoneGroupMember {
            uuid: "RINCON_123456".to_string(),
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            zone_name: "".to_string(), // Empty zone name
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![satellite_with_empty_name],
        };

        let zone_group = ZoneGroup {
            coordinator: "RINCON_123456".to_string(),
            id: "RINCON_123456:1234567890".to_string(),
            members: vec![member_with_empty_name],
        };

        let topology = SonosTopology {
            zone_groups: vec![zone_group],
            vanished_devices: None,
        };

        let (items, _display_strings) = TopologyFlattener::flatten_sonos_topology(&topology);
        
        // Should have 3 items: 1 group + 1 speaker + 1 satellite
        assert_eq!(items.len(), 3);
        
        // Group should use fallback name from group ID
        match &items[0] {
            HierarchicalItem::Group { name, .. } => {
                assert!(name.starts_with("Group ("));
            }
            _ => panic!("Expected Group at index 0"),
        }
        
        // Speaker should have placeholder name with UUID prefix
        match &items[1] {
            HierarchicalItem::Speaker { name, .. } => {
                assert!(name.starts_with("Unknown Speaker (RINCON_1"));
            }
            _ => panic!("Expected Speaker at index 1"),
        }
        
        // Satellite should have placeholder name with UUID prefix
        match &items[2] {
            HierarchicalItem::Satellite { name, .. } => {
                assert!(name.starts_with("Unknown Satellite (RINCON_S"));
            }
            _ => panic!("Expected Satellite at index 2"),
        }
    }

    #[test]
    fn test_visual_formatting_indentation() {
        // Test proper indentation levels for different item types
        let group_display = TopologyFlattener::format_group(&create_test_group());
        assert_eq!(group_display, "Group: Living Room (1 speaker)");

        let speaker_display = TopologyFlattener::format_speaker("Living Room", 1);
        assert_eq!(speaker_display, "  Speaker: Living Room");

        let satellite_display = TopologyFlattener::format_satellite("Surround Left", 2);
        assert_eq!(satellite_display, "    Satellite: Surround Left");
    }

    #[test]
    fn test_visual_formatting_plural_handling() {
        // Test singular vs plural speaker text
        let single_group = Group {
            name: "Living Room".to_string(),
            speakers: vec!["Living Room".to_string()],
        };
        let single_display = TopologyFlattener::format_group(&single_group);
        assert_eq!(single_display, "Group: Living Room (1 speaker)");

        let multi_group = Group {
            name: "Living Room".to_string(),
            speakers: vec!["Living Room".to_string(), "Kitchen".to_string()],
        };
        let multi_display = TopologyFlattener::format_group(&multi_group);
        assert_eq!(multi_display, "Group: Living Room (2 speakers)");
    }
}

#[cfg(test)]
mod topology_list_tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use crate::types::{Topology, Group};

    fn create_test_group() -> Group {
        Group {
            name: "Living Room".to_string(),
            speakers: vec!["Living Room".to_string()],
        }
    }

    fn create_test_topology() -> Topology {
        Topology {
            groups: vec![create_test_group()],
        }
    }

    fn create_multi_group_topology() -> Topology {
        Topology {
            groups: vec![
                Group {
                    name: "Living Room".to_string(),
                    speakers: vec!["Living Room".to_string(), "Kitchen".to_string()],
                },
                Group {
                    name: "Bedroom".to_string(),
                    speakers: vec!["Bedroom".to_string()],
                },
            ],
        }
    }

    fn create_empty_topology() -> Topology {
        Topology {
            groups: vec![],
        }
    }

    #[test]
    fn test_topology_list_creation_from_topology() {
        let topology = create_test_topology();
        let topology_list = TopologyList::new(&topology);

        // Should have 2 items: 1 group + 1 speaker
        assert_eq!(topology_list.len(), 2);
        assert!(!topology_list.is_empty());
        
        // First selection should be the group
        assert_eq!(topology_list.selected(), Some(0));
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Group));
    }

    #[test]
    fn test_topology_list_creation_from_empty_topology() {
        let topology = create_empty_topology();
        let topology_list = TopologyList::new(&topology);

        assert_eq!(topology_list.len(), 0);
        assert!(topology_list.is_empty());
        assert_eq!(topology_list.selected(), None);
        assert_eq!(topology_list.selected_item_type(), None);
    }

    #[test]
    fn test_topology_list_navigation() {
        let topology = create_test_topology();
        let mut topology_list = TopologyList::new(&topology);

        // Start at group (index 0)
        assert_eq!(topology_list.selected(), Some(0));
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Group));

        // Move to speaker (index 1)
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(1));
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Speaker));

        // Wrap to beginning (index 0)
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(0));
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Group));
    }

    #[test]
    fn test_topology_list_selected_item_details() {
        let topology = create_test_topology();
        let mut topology_list = TopologyList::new(&topology);

        // Test group selection
        assert_eq!(topology_list.selected(), Some(0));
        if let Some(HierarchicalItem::Group { name, member_count }) = topology_list.selected_item() {
            assert_eq!(name, "Living Room");
            assert_eq!(*member_count, 1);
        } else {
            panic!("Expected Group item at index 0");
        }

        // Test speaker selection
        topology_list.next();
        assert_eq!(topology_list.selected(), Some(1));
        if let Some(HierarchicalItem::Speaker { name, group_name, is_coordinator }) = topology_list.selected_item() {
            assert_eq!(name, "Living Room");
            assert_eq!(group_name, "Living Room");
            assert!(*is_coordinator);
        } else {
            panic!("Expected Speaker item at index 1");
        }
    }

    #[test]
    fn test_topology_list_multi_group_navigation() {
        let topology = create_multi_group_topology();
        let mut topology_list = TopologyList::new(&topology);

        // Should have 5 items: 2 groups + 3 speakers
        assert_eq!(topology_list.len(), 5);

        // Navigate through all items and verify types
        let expected_types = vec![
            ItemType::Group,   // First group
            ItemType::Speaker, // Living Room (coordinator)
            ItemType::Speaker, // Kitchen (member)
            ItemType::Group,   // Second group
            ItemType::Speaker, // Bedroom (coordinator)
        ];

        for (i, expected_type) in expected_types.iter().enumerate() {
            assert_eq!(topology_list.selected(), Some(i));
            assert_eq!(topology_list.selected_item_type(), Some(*expected_type));
            topology_list.next();
        }

        // Should wrap back to first item
        assert_eq!(topology_list.selected(), Some(0));
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Group));
    }

    #[test]
    fn test_selected_item_error_handling_edge_cases() {
        // Test with empty topology
        let empty_topology = create_empty_topology();
        let empty_list = TopologyList::new(&empty_topology);
        
        // Test basic properties
        assert_eq!(empty_list.len(), 0);
        assert!(empty_list.is_empty());
        
        // Test selection methods return None for empty list
        assert!(empty_list.selected_item().is_none());
        assert!(empty_list.selected_item_type().is_none());
        assert_eq!(empty_list.selected(), None);
    }

    #[test]
    fn test_topology_list_malformed_data_handling() {
        // Test with malformed topology data
        let malformed_topology = Topology {
            groups: vec![
                Group {
                    name: "".to_string(), // Empty name
                    speakers: vec!["Valid Speaker".to_string(), "".to_string()], // Mixed valid/invalid
                },
                Group {
                    name: "Valid Group".to_string(),
                    speakers: vec![], // Empty speakers - should be skipped
                },
                Group {
                    name: "Another Group".to_string(),
                    speakers: vec!["Speaker A".to_string()],
                },
            ],
        };

        let topology_list = TopologyList::new(&malformed_topology);
        
        // Should have items from valid groups only
        // First group: 1 group + 2 speakers (including placeholder for empty name)
        // Second group: skipped due to empty speakers
        // Third group: 1 group + 1 speaker
        // Total: 5 items
        assert_eq!(topology_list.len(), 5);
        assert!(!topology_list.is_empty());
        
        // Should be able to navigate through all items without panicking
        let mut test_list = topology_list;
        for _ in 0..test_list.len() {
            assert!(test_list.selected_item().is_some());
            assert!(test_list.selected_item_type().is_some());
            test_list.next();
        }
        
        // Should wrap back to first item
        assert_eq!(test_list.selected(), Some(0));
    }

    #[test]
    fn test_topology_list_missing_coordinator_handling() {
        // Test topology where coordinator doesn't exist in speakers list
        let topology_missing_coordinator = Topology {
            groups: vec![
                Group {
                    name: "Nonexistent Coordinator".to_string(),
                    speakers: vec!["Speaker 1".to_string(), "Speaker 2".to_string()],
                },
            ],
        };

        let topology_list = TopologyList::new(&topology_missing_coordinator);
        
        // Should have 3 items: 1 group + 2 speakers
        assert_eq!(topology_list.len(), 3);
        
        // Should handle selection gracefully
        assert!(topology_list.selected_item().is_some());
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Group));
        
        // Navigate to speakers and verify one is marked as coordinator (fallback)
        let mut test_list = topology_list;
        test_list.next(); // Move to first speaker
        
        if let Some(HierarchicalItem::Speaker { is_coordinator, .. }) = test_list.selected_item() {
            assert!(*is_coordinator, "First speaker should be marked as coordinator when original is missing");
        } else {
            panic!("Expected Speaker item");
        }
    }

    #[test]
    fn test_sonos_topology_list_error_recovery() {
        use crate::types::{SonosTopology, ZoneGroup, ZoneGroupMember};

        // Test with various edge cases in SonosTopology
        let problematic_member = ZoneGroupMember {
            uuid: "RINCON_123456".to_string(),
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            zone_name: "   ".to_string(), // Whitespace-only name
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![],
        };

        let zone_group_with_issues = ZoneGroup {
            coordinator: "RINCON_MISSING".to_string(), // Non-existent coordinator
            id: "RINCON_MISSING:123".to_string(),
            members: vec![problematic_member],
        };

        let problematic_topology = SonosTopology {
            zone_groups: vec![zone_group_with_issues],
            vanished_devices: None,
        };

        let topology_list = TopologyList::from_sonos_topology(&problematic_topology);
        
        // Should handle the problematic data gracefully
        assert_eq!(topology_list.len(), 2); // 1 group + 1 speaker
        assert!(!topology_list.is_empty());
        
        // Should be able to select items without panicking
        assert!(topology_list.selected_item().is_some());
        assert!(topology_list.selected_item_type().is_some());
        
        // Group should have fallback name
        if let Some(HierarchicalItem::Group { name, .. }) = topology_list.selected_item() {
            assert!(name.starts_with("Group (") || !name.trim().is_empty());
        } else {
            panic!("Expected Group item");
        }
    }

    #[test]
    fn test_edge_case_navigation_consistency() {
        // Test that navigation remains consistent even with edge cases
        let edge_case_topology = Topology {
            groups: vec![
                Group {
                    name: "".to_string(), // Empty name - gets placeholder
                    speakers: vec!["Speaker 1".to_string()],
                },
                Group {
                    name: "Normal Group".to_string(),
                    speakers: vec!["Speaker 2".to_string(), "Speaker 3".to_string()],
                },
            ],
        };

        let mut topology_list = TopologyList::new(&edge_case_topology);
        let total_items = topology_list.len();
        
        // Navigate through all items and back
        for i in 0..total_items * 2 {
            let expected_index = i % total_items;
            assert_eq!(topology_list.selected(), Some(expected_index));
            assert!(topology_list.selected_item().is_some());
            assert!(topology_list.selected_item_type().is_some());
            topology_list.next();
        }
        
        // Test reverse navigation
        for i in 0..total_items {
            topology_list.previous();
            let expected_index = (total_items - 1 - i) % total_items;
            assert_eq!(topology_list.selected(), Some(expected_index));
            assert!(topology_list.selected_item().is_some());
        }
    }

    #[test]
    fn test_sonos_topology_list_creation_with_satellites() {
        use crate::types::{SonosTopology, ZoneGroup, ZoneGroupMember, Satellite};

        let satellite = Satellite {
            uuid: "RINCON_SAT123".to_string(),
            location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
            zone_name: "Surround Left".to_string(),
            software_version: "56.0-76060".to_string(),
        };

        let member = ZoneGroupMember {
            uuid: "RINCON_123456".to_string(),
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            zone_name: "Living Room".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![satellite],
        };

        let zone_group = ZoneGroup {
            coordinator: "RINCON_123456".to_string(),
            id: "RINCON_123456:1234567890".to_string(),
            members: vec![member],
        };

        let topology = SonosTopology {
            zone_groups: vec![zone_group],
            vanished_devices: None,
        };

        let topology_list = TopologyList::from_sonos_topology(&topology);

        // Should have 3 items: 1 group + 1 speaker + 1 satellite
        assert_eq!(topology_list.len(), 3);
        assert!(!topology_list.is_empty());
        
        // First selection should be the group
        assert_eq!(topology_list.selected(), Some(0));
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Group));
    }

    #[test]
    fn test_sonos_topology_navigation_with_satellites() {
        use crate::types::{SonosTopology, ZoneGroup, ZoneGroupMember, Satellite};

        let satellite = Satellite {
            uuid: "RINCON_SAT123".to_string(),
            location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
            zone_name: "Surround Left".to_string(),
            software_version: "56.0-76060".to_string(),
        };

        let member = ZoneGroupMember {
            uuid: "RINCON_123456".to_string(),
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            zone_name: "Living Room".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![satellite],
        };

        let zone_group = ZoneGroup {
            coordinator: "RINCON_123456".to_string(),
            id: "RINCON_123456:1234567890".to_string(),
            members: vec![member],
        };

        let topology = SonosTopology {
            zone_groups: vec![zone_group],
            vanished_devices: None,
        };

        let mut topology_list = TopologyList::from_sonos_topology(&topology);

        // Navigate through all items and verify types
        let expected_types = vec![
            ItemType::Group,     // Zone group
            ItemType::Speaker,   // Living Room speaker
            ItemType::Satellite, // Surround Left satellite
        ];

        for (i, expected_type) in expected_types.iter().enumerate() {
            assert_eq!(topology_list.selected(), Some(i));
            assert_eq!(topology_list.selected_item_type(), Some(*expected_type));
            topology_list.next();
        }

        // Should wrap back to first item
        assert_eq!(topology_list.selected(), Some(0));
        assert_eq!(topology_list.selected_item_type(), Some(ItemType::Group));
    }

    #[test]
    fn test_topology_list_draw_doesnt_panic() {
        let topology = create_test_topology();
        let mut topology_list = TopologyList::new(&topology);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Should not panic when drawing
        terminal.draw(|frame| {
            topology_list.draw(frame, frame.area());
        }).unwrap();
    }

    #[test]
    fn test_sonos_topology_flattening_with_satellites() {
        use crate::types::{SonosTopology, ZoneGroup, ZoneGroupMember, Satellite};

        // Create a test topology with satellites
        let satellite = Satellite {
            uuid: "RINCON_SAT123".to_string(),
            location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
            zone_name: "Surround Left".to_string(),
            software_version: "56.0-76060".to_string(),
        };

        let member = ZoneGroupMember {
            uuid: "RINCON_123456".to_string(),
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            zone_name: "Living Room".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![satellite],
        };

        let zone_group = ZoneGroup {
            coordinator: "RINCON_123456".to_string(),
            id: "RINCON_123456:1234567890".to_string(),
            members: vec![member],
        };

        let topology = SonosTopology {
            zone_groups: vec![zone_group],
            vanished_devices: None,
        };

        let (items, display_strings) = TopologyFlattener::flatten_sonos_topology(&topology);

        // Should have: 1 group + 1 speaker + 1 satellite = 3 items
        assert_eq!(items.len(), 3);
        assert_eq!(display_strings.len(), 3);

        // Check the group
        match &items[0] {
            HierarchicalItem::Group { name, member_count } => {
                assert_eq!(name, "Living Room");
                assert_eq!(*member_count, 1);
            }
            _ => panic!("Expected Group at index 0"),
        }
        assert_eq!(display_strings[0], "Group: Living Room (1 speaker, 1 satellite)");

        // Check the speaker
        match &items[1] {
            HierarchicalItem::Speaker { name, group_name, is_coordinator } => {
                assert_eq!(name, "Living Room");
                assert_eq!(group_name, "Living Room");
                assert!(*is_coordinator);
            }
            _ => panic!("Expected Speaker at index 1"),
        }
        assert_eq!(display_strings[1], "  Speaker: Living Room");

        // Check the satellite
        match &items[2] {
            HierarchicalItem::Satellite { name, parent_speaker_name, group_name } => {
                assert_eq!(name, "Surround Left");
                assert_eq!(parent_speaker_name, "Living Room");
                assert_eq!(group_name, "Living Room");
            }
            _ => panic!("Expected Satellite at index 2"),
        }
        assert_eq!(display_strings[2], "    Satellite: Surround Left");
    }

    #[test]
    fn test_zone_group_formatting_with_satellites() {
        use crate::types::{ZoneGroup, ZoneGroupMember, Satellite};

        // Test zone group with satellites
        let satellite1 = Satellite {
            uuid: "RINCON_SAT1".to_string(),
            location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
            zone_name: "Surround Left".to_string(),
            software_version: "56.0-76060".to_string(),
        };

        let satellite2 = Satellite {
            uuid: "RINCON_SAT2".to_string(),
            location: "http://192.168.1.102:1400/xml/device_description.xml".to_string(),
            zone_name: "Surround Right".to_string(),
            software_version: "56.0-76060".to_string(),
        };

        let member = ZoneGroupMember {
            uuid: "RINCON_123456".to_string(),
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            zone_name: "Living Room".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![satellite1, satellite2],
        };

        let zone_group = ZoneGroup {
            coordinator: "RINCON_123456".to_string(),
            id: "RINCON_123456:1234567890".to_string(),
            members: vec![member],
        };

        let display = TopologyFlattener::format_zone_group(&zone_group, "Living Room");
        assert_eq!(display, "Group: Living Room (1 speaker, 2 satellites)");

        // Test zone group without satellites
        let member_no_satellites = ZoneGroupMember {
            uuid: "RINCON_789".to_string(),
            location: "http://192.168.1.103:1400/xml/device_description.xml".to_string(),
            zone_name: "Kitchen".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:kitchen".to_string(),
            satellites: vec![],
        };

        let zone_group_no_satellites = ZoneGroup {
            coordinator: "RINCON_789".to_string(),
            id: "RINCON_789:987".to_string(),
            members: vec![member_no_satellites],
        };

        let display_no_satellites = TopologyFlattener::format_zone_group(&zone_group_no_satellites, "Kitchen");
        assert_eq!(display_no_satellites, "Group: Kitchen (1 speaker)");
    }}
