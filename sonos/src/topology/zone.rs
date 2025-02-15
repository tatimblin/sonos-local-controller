pub struct Zone {
  name: String,
  speakers: Vec<String>,
}

impl Zone {
  pub fn new(name: String) -> Zone {
    Self {
      name,
      speakers: Vec::new(),
    }
  }
}