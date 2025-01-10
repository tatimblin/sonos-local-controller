use crate::speaker::model::Action;
use crate::speaker::client::Client;
use crate::SonosError;

#[derive(Debug)]
pub struct ApiClient {
  client: Client,
}

impl ApiClient {
  pub fn new(client: Client) -> Self {
    Self { client }
  }

  pub fn play(&self) -> Result<(), SonosError> {
    let payload = "<InstanceID>0</InstanceID><Speed>1</Speed>";
    self.client.send_action(Action::Play, payload)
      .map(|_| ())
  }

  pub fn pause(&self) -> Result<(), SonosError> {
    let payload = "<InstanceID>0</InstanceID>";
    self.client.send_action(Action::Pause, payload)
      .map(|_| ())
  }

  pub fn get_volume(&self) -> Result<u8, SonosError> {
    let payload = "<InstanceID>0</InstanceID><Channel>Master</Channel>";
    match self.client.send_action(Action::GetVolume, payload) {
      Ok(response) => {
        let volume = self.client.get_child_element_text(&response, "CurrentVolume")?
          .parse::<u8>()
          .map_err(|e| SonosError::ParseError(format!("Failed to parse volume: {}", e)))?;
        Ok(volume)
      },
      Err(error) => Err(error),
    }
  }
}
