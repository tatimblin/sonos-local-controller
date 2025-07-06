use std::fmt;
use sonos::SonosError;

/// Errors that can occur during command execution
#[derive(Debug)]
pub enum CommandError {
    /// Command does not support the selected target type (speaker/group)
    UnsupportedTarget(String),
    /// No item is currently selected when command is executed
    NoSelection,
    /// Error from the underlying Sonos library
    SonosError(SonosError),
    /// Speaker could not be found by the given identifier
    SpeakerNotFound(String),
    /// Group coordinator could not be found
    CoordinatorNotFound(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandError::UnsupportedTarget(msg) => {
                write!(f, "Command not supported for this target: {}", msg)
            }
            CommandError::NoSelection => {
                write!(f, "No item selected")
            }
            CommandError::SonosError(err) => {
                write!(f, "Sonos error: {}", err)
            }
            CommandError::SpeakerNotFound(identifier) => {
                write!(f, "Speaker not found: {}", identifier)
            }
            CommandError::CoordinatorNotFound(group_name) => {
                write!(f, "Coordinator not found for group: {}", group_name)
            }
        }
    }
}

impl std::error::Error for CommandError {}

impl From<SonosError> for CommandError {
    fn from(err: SonosError) -> Self {
        CommandError::SonosError(err)
    }
}

/// Result type for command execution
pub type CommandResult = Result<String, CommandError>;