use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SpeakerId(String);

impl<'de> Deserialize<'de> for SpeakerId {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let id = String::deserialize(deserializer)?;
    Ok(Self::new(id))
  }
}

impl SpeakerId {
  /// Creates a new SpeakerId, stripping the "uuid:" prefix if present
  pub fn new(id: impl Into<String>) -> Self {
    let id = id.into();
    let normalized = id.strip_prefix("uuid:").unwrap_or(&id);
    Self(normalized.to_string())
  }

  /// Returns the ID as a string slice
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl std::fmt::Display for SpeakerId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl AsRef<str> for SpeakerId {
  fn as_ref(&self) -> &str {
    &self.0
  }
}
