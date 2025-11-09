#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GroupId(String);

impl GroupId {
  pub fn new(id: impl Into<String>) -> Self {
    GroupId(id.into())
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }
}