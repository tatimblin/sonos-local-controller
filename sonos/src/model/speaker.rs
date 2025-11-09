use crate::SpeakerId;

#[derive(Debug, Clone)]
pub struct Speaker {
    pub id: SpeakerId,
    pub name: String,
    pub room_name: String,
    pub ip_address: String,
    pub port: u16,
    pub model_name: String,
    pub satellites: Vec<SpeakerId>,
}

impl Speaker {
  pub fn get_id(&self) -> &SpeakerId {
    &self.id
  }
}