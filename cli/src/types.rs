#[derive(Clone, Copy, Debug, PartialEq)]
pub enum View {
  Startup,
  Control,
}

#[derive(Debug, Clone)]
pub struct Topology {
    pub groups: Vec<Group>,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,        // Name of the coordinator speaker
    pub speakers: Vec<String>, // Names of all speakers in the group
}
