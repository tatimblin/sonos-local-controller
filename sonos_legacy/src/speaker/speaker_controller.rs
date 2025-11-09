use log::info;

use crate::client::Client;
use crate::error::SonosError;
use crate::model::{Action, PlayState};
use crate::speaker::{Device, SpeakerInfo};
use crate::{ZoneGroup, ZoneGroupMember};

/// A stateless Sonos speaker controller that operates on a specific IP address
#[derive(Debug, Clone)]
pub struct SpeakerController {
    client: Client,
}

impl SpeakerController {
    /// Create a new Speaker controller bound to the specified IP address
    pub fn new() -> Self {
        Self {
            client: Client::default(),
        }
    }

    /// Create a new Speaker controller with a custom HTTP client
    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    /// Get a reference to the HTTP client used by this speaker
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get detailed information about this speaker
    pub fn get_info(&self, ip: &str) -> Result<SpeakerInfo, SonosError> {
        SpeakerInfo::from_location(ip)
    }

    /// Get the current playback state of this speaker
    pub fn get_play_state(&self, ip: &str) -> Result<PlayState, SonosError> {
        let payload = "<InstanceID>0</InstanceID>";
        let response = self
            .client
            .send_action(ip, Action::GetTransportInfo, payload)?;

        let transport_state = self
            .client
            .get_child_element_text(&response, "CurrentTransportState")?;

        Ok(PlayState::from_transport_state(&transport_state))
    }

    /// Start playback on this speaker
    pub fn play(&self, ip: &str) -> Result<(), SonosError> {
        if !self.is_coordinator(ip)? {
            return Err(SonosError::NotCoordinator(ip.to_string()));
        }

        let payload = "<InstanceID>0</InstanceID><Speed>1</Speed>";
        self.client.send_action(ip, Action::Play, payload)?;
        Ok(())
    }

    /// Pause playback on this speaker
    pub fn pause(&self, ip: &str) -> Result<(), SonosError> {
        if !self.is_coordinator(ip)? {
            return Err(SonosError::NotCoordinator(ip.to_string()));
        }

        let payload = "<InstanceID>0</InstanceID>";
        self.client.send_action(ip, Action::Pause, payload)?;
        Ok(())
    }

    /// Stop playback on this speaker
    pub fn stop(&self, ip: &str) -> Result<(), SonosError> {
        if !self.is_coordinator(ip)? {
            return Err(SonosError::NotCoordinator(ip.to_string()));
        }

        let payload = "<InstanceID>0</InstanceID>";
        self.client.send_action(ip, Action::Stop, payload)?;
        Ok(())
    }

    pub fn toggle_play_state(&self, ip: &str) -> Result<(), SonosError> {
        match self.get_play_state(ip)? {
            PlayState::Playing | PlayState::Transitioning => self.pause(ip),
            PlayState::Paused | PlayState::Stopped => self.play(ip),
        }
    }

    /// Get the current volume level (0-100)
    pub fn get_volume(&self, ip: &str) -> Result<u8, SonosError> {
        let payload = "<InstanceID>0</InstanceID><Channel>Master</Channel>";
        let response = self.client.send_action(ip, Action::GetVolume, payload)?;
        self.parse_element_u8(&response, "CurrentVolume")
    }

    pub fn get_group_volume(&self, ip: &str) -> Result<u8, SonosError> {
        let payload = "<InstanceID>0</InstanceID>";
        let response = self
            .client
            .send_action(ip, Action::GetGroupVolume, payload)?;
        self.parse_element_u8(&response, "CurrentVolume")
    }

    /// Set the volume level (0-100)
    pub fn set_volume(&self, ip: &str, volume: u8) -> Result<(), SonosError> {
        if volume > 100 {
            return Err(SonosError::InvalidVolume(volume));
        }

        let payload = format!(
            "<InstanceID>0</InstanceID><Channel>Master</Channel><DesiredVolume>{}</DesiredVolume>",
            volume
        );
        self.client.send_action(ip, Action::SetVolume, &payload)?;
        Ok(())
    }

    /// Adjust the volume by a relative amount (-100 to +100)
    pub fn adjust_volume(&self, ip: &str, adjustment: i8) -> Result<u8, SonosError> {
        let payload = format!(
            "<InstanceID>0</InstanceID><Channel>Master</Channel><Adjustment>{}</Adjustment>",
            adjustment
        );
        let response = self
            .client
            .send_action(ip, Action::SetRelativeVolume, &payload)?;
        self.parse_element_u8(&response, "NewVolume")
    }

    fn is_coordinator(&self, ip: &str) -> Result<bool, SonosError> {
        let payload = "<InstanceID>0</InstanceID>";
        let response = self
            .client
            .send_action(ip, Action::GetPositionInfo, payload)?;

        let rel_time = self.client.get_child_element_text(&response, "RelTime")?;

        Ok(!rel_time.is_empty())
    }

    /// Parse a u8 value from an XML element
    fn parse_element_u8(&self, element: &xmltree::Element, key: &str) -> Result<u8, SonosError> {
        self.client
            .get_child_element_text(element, key)?
            .parse()
            .map_err(|e| SonosError::ParseError(format!("Failed to parse {}: {}", key, e)))
    }
}
