use sonos::{SpeakerTrait, SpeakerFactory, Speaker};
use crate::commands::{Command, CommandResult, CommandError};
use crate::widgets::topology_list::HierarchicalItem;
use crate::types::{Topology, Group, SpeakerInfo};

/// Routes commands to the appropriate speakers based on the target type and topology
/// 
/// The CommandRouter handles the logic of determining which speaker should receive
/// a command based on the selected target (group, speaker, or satellite) and the
/// current simplified topology. For group commands, it routes to the coordinator speaker.
/// For speaker commands on speakers that are part of groups, it can route to either
/// the speaker directly or the group coordinator depending on the command type.
pub struct CommandRouter {
    topology: Option<Topology>,
}

impl CommandRouter {
    /// Creates a new CommandRouter with no topology loaded
    pub fn new() -> Self {
        Self {
            topology: None,
        }
    }

    /// Creates a new CommandRouter with the given topology
    pub fn with_topology(topology: Topology) -> Self {
        Self {
            topology: Some(topology),
        }
    }

    /// Updates the topology used for command routing
    pub fn set_topology(&mut self, topology: Topology) {
        self.topology = Some(topology);
    }

    /// Executes a command on the appropriate target based on the hierarchical item
    /// 
    /// # Arguments
    /// * `command` - The command to execute
    /// * `target` - The selected hierarchical item (group, speaker, or satellite)
    /// 
    /// # Returns
    /// * `Ok(String)` - Success message from command execution
    /// * `Err(CommandError)` - Error if routing fails or command execution fails
    pub fn execute_command<C: Command>(
        &self,
        command: &C,
        target: &HierarchicalItem,
    ) -> CommandResult {
        let topology = self.topology.as_ref()
            .ok_or(CommandError::UnsupportedTarget("No topology available".to_string()))?;

        match target {
            HierarchicalItem::Group { name, .. } => {
                self.execute_on_group(command, name, topology)
            }
            HierarchicalItem::Speaker { name, group_name, is_coordinator, .. } => {
                self.execute_on_speaker(command, name, group_name, *is_coordinator, topology)
            }
            HierarchicalItem::Satellite { parent_speaker_name, group_name, .. } => {
                // Satellites are treated like their parent speaker
                self.execute_on_speaker(command, parent_speaker_name, group_name, false, topology)
            }
        }
    }

    /// Executes a command on a group by routing to its coordinator
    fn execute_on_group<C: Command>(
        &self,
        command: &C,
        group_name: &str,
        topology: &Topology,
    ) -> CommandResult {
        if !command.supports_group() {
            return Err(CommandError::UnsupportedTarget(
                format!("Command '{}' does not support groups", command.name())
            ));
        }

        let coordinator = self.find_group_coordinator(group_name, topology)?;
        command.execute_on_group(&coordinator)
    }

    /// Executes a command on a speaker, routing to group coordinator if needed
    fn execute_on_speaker<C: Command>(
        &self,
        command: &C,
        speaker_name: &str,
        group_name: &str,
        is_coordinator: bool,
        topology: &Topology,
    ) -> CommandResult {
        // If command supports speakers, execute directly on the speaker
        if command.supports_speaker() {
            let speaker = self.find_speaker_by_name(speaker_name, topology)?;
            return command.execute_on_speaker(&speaker);
        }

        // If command only supports groups and this speaker is part of a group,
        // route to the group coordinator
        if command.supports_group() {
            let coordinator = self.find_group_coordinator(group_name, topology)?;
            return command.execute_on_group(&coordinator);
        }

        // Command doesn't support either speakers or groups
        Err(CommandError::UnsupportedTarget(
            format!("Command '{}' does not support the selected target", command.name())
        ))
    }

    /// Finds the coordinator speaker for a given group
    fn find_group_coordinator(
        &self,
        group_name: &str,
        topology: &Topology,
    ) -> Result<Speaker, CommandError> {
        // Find the group by name
        let group = topology.groups.iter()
            .find(|g| g.name == group_name)
            .ok_or_else(|| CommandError::CoordinatorNotFound(group_name.to_string()))?;

        // Find the coordinator speaker in the group
        let coordinator = group.speakers.iter()
            .find(|speaker| speaker.is_coordinator)
            .ok_or_else(|| CommandError::CoordinatorNotFound(group_name.to_string()))?;

        // Create Speaker from the coordinator's IP address
        let speaker_location = format!("http://{}:1400/xml/device_description.xml", coordinator.ip);
        Speaker::from_location(&speaker_location)
            .map_err(CommandError::from)
    }

    /// Finds a speaker by name in the topology
    fn find_speaker_by_name(
        &self,
        speaker_name: &str,
        topology: &Topology,
    ) -> Result<Speaker, CommandError> {
        // Search through all groups and speakers to find the speaker
        for group in &topology.groups {
            for speaker in &group.speakers {
                if speaker.name == speaker_name {
                    let speaker_location = format!("http://{}:1400/xml/device_description.xml", speaker.ip);
                    return Speaker::from_location(&speaker_location)
                        .map_err(CommandError::from);
                }
            }
        }

        Err(CommandError::SpeakerNotFound(speaker_name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::pause::PauseCommand;

    fn create_test_topology() -> Topology {
        Topology {
            groups: vec![
                Group {
                    name: "Living Room".to_string(),
                    speakers: vec![
                        SpeakerInfo {
                            name: "Living Room".to_string(),
                            uuid: "RINCON_000E58C0123456789".to_string(),
                            ip: "192.168.1.100".to_string(),
                            is_coordinator: true,
                        },
                    ],
                },
            ],
        }
    }

    fn create_test_group_item() -> HierarchicalItem {
        HierarchicalItem::Group {
            name: "Living Room".to_string(),
            member_count: 1,
        }
    }

    fn create_test_speaker_item() -> HierarchicalItem {
        HierarchicalItem::Speaker {
            name: "Living Room".to_string(),
            group_name: "Living Room".to_string(),
            is_coordinator: true,
        }
    }

    #[test]
    fn test_router_creation() {
        let router = CommandRouter::new();
        assert!(router.topology.is_none());

        let topology = create_test_topology();
        let router_with_topology = CommandRouter::with_topology(topology.clone());
        assert!(router_with_topology.topology.is_some());
    }

    #[test]
    fn test_router_set_topology() {
        let mut router = CommandRouter::new();
        assert!(router.topology.is_none());

        let topology = create_test_topology();
        router.set_topology(topology);
        assert!(router.topology.is_some());
    }

    #[test]
    fn test_execute_command_no_topology() {
        let router = CommandRouter::new();
        let command = PauseCommand;
        let target = create_test_group_item();

        let result = router.execute_command(&command, &target);
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::UnsupportedTarget(msg) => {
                assert!(msg.contains("No topology available"));
            }
            _ => panic!("Expected UnsupportedTarget error"),
        }
    }

    #[test]
    fn test_execute_command_on_group_unsupported() {
        let topology = create_test_topology();
        let router = CommandRouter::with_topology(topology);
        
        // Create a mock command that doesn't support groups
        struct UnsupportedCommand;
        impl Command for UnsupportedCommand {
            fn name(&self) -> &'static str { "Unsupported" }
            fn supports_speaker(&self) -> bool { true }
            fn supports_group(&self) -> bool { false }
            fn execute_on_speaker(&self, _: &dyn SpeakerTrait) -> CommandResult { Ok("".to_string()) }
            fn execute_on_group(&self, _: &dyn SpeakerTrait) -> CommandResult { Ok("".to_string()) }
        }

        let command = UnsupportedCommand;
        let target = create_test_group_item();

        let result = router.execute_command(&command, &target);
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::UnsupportedTarget(msg) => {
                assert!(msg.contains("does not support groups"));
            }
            _ => panic!("Expected UnsupportedTarget error"),
        }
    }

    #[test]
    fn test_execute_command_on_speaker_unsupported() {
        let topology = create_test_topology();
        let router = CommandRouter::with_topology(topology);
        
        // Create a mock command that doesn't support speakers or groups
        struct UnsupportedCommand;
        impl Command for UnsupportedCommand {
            fn name(&self) -> &'static str { "Unsupported" }
            fn supports_speaker(&self) -> bool { false }
            fn supports_group(&self) -> bool { false }
            fn execute_on_speaker(&self, _: &dyn SpeakerTrait) -> CommandResult { Ok("".to_string()) }
            fn execute_on_group(&self, _: &dyn SpeakerTrait) -> CommandResult { Ok("".to_string()) }
        }

        let command = UnsupportedCommand;
        let target = create_test_speaker_item();

        let result = router.execute_command(&command, &target);
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::UnsupportedTarget(msg) => {
                assert!(msg.contains("does not support the selected target"));
            }
            _ => panic!("Expected UnsupportedTarget error"),
        }
    }

    #[test]
    fn test_find_group_coordinator_not_found() {
        let topology = create_test_topology();
        let router = CommandRouter::with_topology(topology);

        let result = router.find_group_coordinator("Nonexistent Group", router.topology.as_ref().unwrap());
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::CoordinatorNotFound(name) => {
                assert_eq!(name, "Nonexistent Group");
            }
            _ => panic!("Expected CoordinatorNotFound error"),
        }
    }

    #[test]
    fn test_find_speaker_by_name_not_found() {
        let topology = create_test_topology();
        let router = CommandRouter::with_topology(topology);

        let result = router.find_speaker_by_name("Nonexistent Speaker", router.topology.as_ref().unwrap());
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::SpeakerNotFound(name) => {
                assert_eq!(name, "Nonexistent Speaker");
            }
            _ => panic!("Expected SpeakerNotFound error"),
        }
    }
}