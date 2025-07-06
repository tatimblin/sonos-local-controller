use sonos::SpeakerTrait;
use crate::commands::{Command, CommandResult, CommandError};

/// Command to pause playback on Sonos speakers and groups
/// 
/// This command only supports groups and will pause playback for the entire group
/// by sending the pause command to the group's coordinator speaker. Individual
/// speakers are not supported as pause functionality is designed for group control.
pub struct PauseCommand;

impl Command for PauseCommand {
    fn name(&self) -> &'static str {
        "Pause"
    }

    fn supports_speaker(&self) -> bool {
        false // Pause command only works on groups
    }

    fn supports_group(&self) -> bool {
        true
    }

    fn execute_on_speaker(&self, _speaker: &dyn SpeakerTrait) -> CommandResult {
        // This should never be called since supports_speaker() returns false
        Err(CommandError::UnsupportedTarget(
            "Pause command only works on groups, not individual speakers".to_string()
        ))
    }

    fn execute_on_group(&self, coordinator: &dyn SpeakerTrait) -> CommandResult {
        match coordinator.pause() {
            Ok(()) => Ok(format!("Paused group: {}", coordinator.name())),
            Err(sonos_error) => Err(CommandError::from(sonos_error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sonos::speaker::mock::{MockSpeaker, MockSpeakerBuilder};
    use sonos::SonosError;

    #[test]
    fn test_pause_command_properties() {
        let command = PauseCommand;
        
        assert_eq!(command.name(), "Pause");
        assert!(!command.supports_speaker());
        assert!(command.supports_group());
    }

    #[test]
    fn test_pause_command_execute_on_speaker_returns_error() {
        let command = PauseCommand;
        let mock_speaker = MockSpeakerBuilder::new()
            .name("Test Speaker")
            .build();
        
        let result = command.execute_on_speaker(&mock_speaker);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::UnsupportedTarget(msg) => {
                assert!(msg.contains("only works on groups"));
            }
            _ => panic!("Expected UnsupportedTarget error"),
        }
    }

    #[test]
    fn test_pause_command_execute_on_group_success() {
        let command = PauseCommand;
        let mock_speaker = MockSpeakerBuilder::new()
            .name("Living Room")
            .build();
        
        let result = command.execute_on_group(&mock_speaker);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Paused group: Living Room");
    }

    #[test]
    fn test_pause_command_execute_on_group_failure() {
        let command = PauseCommand;
        let mut mock_speaker = MockSpeaker::new();
        
        mock_speaker.expect_name().return_const("Living Room".to_string());
        mock_speaker.expect_pause().returning(|| {
            Err(SonosError::DeviceUnreachable)
        });
        
        let result = command.execute_on_group(&mock_speaker);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::SonosError(SonosError::DeviceUnreachable) => {
                // Expected error type
            }
            _ => panic!("Expected SonosError::DeviceUnreachable"),
        }
    }
}