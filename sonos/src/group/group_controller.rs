pub struct GroupController {
  speaker_controller: SpeakerController::new(),
}

impl GroupController {
  pub fn play(&self, coordinator_ip: &str) -> Result<(), SonosError> {
    self.speaker_controller.play(coordinator_ip)
  }

  pub fn pause(&self, coordinator_ip: &str) -> Result<(), SonosError> {
    self.speaker_controller.pause(coordinator_ip)
  }
}
