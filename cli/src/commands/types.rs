use sonos::{Speaker, SpeakerTrait};
use crate::commands::error::CommandResult;

/// Trait defining the interface for all commands in the system
/// 
/// Commands implement this trait to provide extensible control functionality
/// that can work on both individual speakers and groups. The trait allows
/// commands to specify their capabilities and provides separate execution
/// methods for different target types.
pub trait Command {
    /// Returns the human-readable name of this command
    fn name(&self) -> &'static str;
    
    /// Returns true if this command can be executed on individual speakers
    fn supports_speaker(&self) -> bool;
    
    /// Returns true if this command can be executed on groups
    fn supports_group(&self) -> bool;
    
    /// Executes the command on an individual speaker
    /// 
    /// # Arguments
    /// * `speaker` - The speaker to execute the command on
    /// 
    /// # Returns
    /// * `Ok(String)` - Success message describing the action performed
    /// * `Err(CommandError)` - Error if the command failed
    /// 
    /// # Note
    /// This method should only be called if `supports_speaker()` returns true
    fn execute_on_speaker(&self, speaker: &dyn SpeakerTrait) -> CommandResult;
    
    /// Executes the command on a group via its coordinator speaker
    /// 
    /// # Arguments
    /// * `coordinator` - The coordinator speaker of the group
    /// 
    /// # Returns
    /// * `Ok(String)` - Success message describing the action performed
    /// * `Err(CommandError)` - Error if the command failed
    /// 
    /// # Note
    /// This method should only be called if `supports_group()` returns true
    fn execute_on_group(&self, coordinator: &dyn SpeakerTrait) -> CommandResult;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::error::CommandError;
    use sonos::Speaker;

    // Mock command for testing
    struct TestCommand {
        supports_speaker: bool,
        supports_group: bool,
    }

    impl Command for TestCommand {
        fn name(&self) -> &'static str {
            "Test Command"
        }

        fn supports_speaker(&self) -> bool {
            self.supports_speaker
        }

        fn supports_group(&self) -> bool {
            self.supports_group
        }

        fn execute_on_speaker(&self, _speaker: &dyn SpeakerTrait) -> CommandResult {
            if self.supports_speaker {
                Ok("Executed on speaker".to_string())
            } else {
                Err(CommandError::UnsupportedTarget("Speaker not supported".to_string()))
            }
        }

        fn execute_on_group(&self, _coordinator: &dyn SpeakerTrait) -> CommandResult {
            if self.supports_group {
                Ok("Executed on group".to_string())
            } else {
                Err(CommandError::UnsupportedTarget("Group not supported".to_string()))
            }
        }
    }

    #[test]
    fn test_command_trait_basic_functionality() {
        let speaker_only_command = TestCommand {
            supports_speaker: true,
            supports_group: false,
        };

        let group_only_command = TestCommand {
            supports_speaker: false,
            supports_group: true,
        };

        let both_command = TestCommand {
            supports_speaker: true,
            supports_group: true,
        };

        // Test command properties
        assert_eq!(speaker_only_command.name(), "Test Command");
        assert!(speaker_only_command.supports_speaker());
        assert!(!speaker_only_command.supports_group());

        assert!(!group_only_command.supports_speaker());
        assert!(group_only_command.supports_group());

        assert!(both_command.supports_speaker());
        assert!(both_command.supports_group());
    }

    #[test]
    fn test_command_error_display() {
        let error = CommandError::UnsupportedTarget("Test target".to_string());
        assert_eq!(format!("{}", error), "Command not supported for this target: Test target");

        let error = CommandError::NoSelection;
        assert_eq!(format!("{}", error), "No item selected");

        let error = CommandError::SpeakerNotFound("Living Room".to_string());
        assert_eq!(format!("{}", error), "Speaker not found: Living Room");

        let error = CommandError::CoordinatorNotFound("Living Room Group".to_string());
        assert_eq!(format!("{}", error), "Coordinator not found for group: Living Room Group");
    }
}