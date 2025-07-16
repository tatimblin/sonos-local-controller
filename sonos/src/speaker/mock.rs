//! Mock speaker implementation for testing network scenarios
//!
//! This module provides mock implementations of speakers that can simulate
//! various network conditions, failures, and edge cases for comprehensive testing.

use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::speaker::SpeakerTrait;
use crate::error::SonosError;

/// Configuration for mock speaker behavior
#[derive(Debug, Clone)]
pub struct MockSpeakerConfig {
    /// Whether the speaker should simulate being unreachable
    pub simulate_unreachable: bool,
    /// Whether the speaker should simulate network timeouts
    pub simulate_timeout: bool,
    /// Whether the speaker should simulate parse errors
    pub simulate_parse_error: bool,
    /// Delay in milliseconds before responding
    pub response_delay_ms: u64,
    /// Whether commands should succeed or fail
    pub commands_succeed: bool,
    /// Custom error message to return
    pub custom_error_message: Option<String>,
}

impl Default for MockSpeakerConfig {
    fn default() -> Self {
        Self {
            simulate_unreachable: false,
            simulate_timeout: false,
            simulate_parse_error: false,
            response_delay_ms: 0,
            commands_succeed: true,
            custom_error_message: None,
        }
    }
}

/// Mock speaker implementation for testing
#[derive(Debug, Clone)]
pub struct MockSpeaker {
    name: String,
    room_name: String,
    ip: String,
    uuid: String,
    volume: Arc<Mutex<u8>>,
    config: Arc<Mutex<MockSpeakerConfig>>,
    command_history: Arc<Mutex<Vec<String>>>,
}

impl MockSpeaker {
    /// Create a new mock speaker with default configuration
    pub fn new(name: &str, uuid: &str, ip: &str) -> Self {
        Self {
            name: name.to_string(),
            room_name: name.to_string(),
            ip: ip.to_string(),
            uuid: uuid.to_string(),
            volume: Arc::new(Mutex::new(50)), // Default volume
            config: Arc::new(Mutex::new(MockSpeakerConfig::default())),
            command_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Update the mock configuration
    pub fn set_config(&self, config: MockSpeakerConfig) {
        *self.config.lock().unwrap() = config;
    }

    /// Get the command history for testing
    pub fn get_command_history(&self) -> Vec<String> {
        self.command_history.lock().unwrap().clone()
    }

    /// Clear the command history
    pub fn clear_command_history(&self) {
        self.command_history.lock().unwrap().clear();
    }

    /// Simulate network delay if configured
    fn simulate_delay(&self) {
        let config = self.config.lock().unwrap();
        if config.response_delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(config.response_delay_ms));
        }
    }

    /// Check if we should simulate an error and return it
    fn check_for_simulated_error(&self, command: &str) -> Result<(), SonosError> {
        let config = self.config.lock().unwrap();
        
        if config.simulate_unreachable {
            return Err(SonosError::DeviceUnreachable);
        }
        
        if config.simulate_timeout {
            return Err(SonosError::NetworkTimeout);
        }
        
        if config.simulate_parse_error {
            return Err(SonosError::ParseError(format!("Mock parse error for {}", command)));
        }
        
        if !config.commands_succeed {
            if let Some(ref custom_message) = config.custom_error_message {
                return Err(SonosError::ParseError(custom_message.clone()));
            } else {
                return Err(SonosError::DeviceUnreachable);
            }
        }
        
        Ok(())
    }

    /// Record a command in the history
    fn record_command(&self, command: &str) {
        self.command_history.lock().unwrap().push(command.to_string());
    }
}

impl SpeakerTrait for MockSpeaker {
    fn name(&self) -> &str {
        &self.name
    }

    fn room_name(&self) -> &str {
        &self.room_name
    }

    fn ip(&self) -> &str {
        &self.ip
    }

    fn uuid(&self) -> &str {
        &self.uuid
    }

    fn play(&self) -> Result<(), SonosError> {
        self.simulate_delay();
        self.check_for_simulated_error("play")?;
        self.record_command("play");
        Ok(())
    }

    fn pause(&self) -> Result<(), SonosError> {
        self.simulate_delay();
        self.check_for_simulated_error("pause")?;
        self.record_command("pause");
        Ok(())
    }

    fn get_volume(&self) -> Result<u8, SonosError> {
        self.simulate_delay();
        self.check_for_simulated_error("get_volume")?;
        self.record_command("get_volume");
        Ok(*self.volume.lock().unwrap())
    }

    fn set_volume(&self, volume: u8) -> Result<u8, SonosError> {
        self.simulate_delay();
        self.check_for_simulated_error("set_volume")?;
        self.record_command(&format!("set_volume({})", volume));
        *self.volume.lock().unwrap() = volume;
        Ok(volume)
    }

    fn adjust_volume(&self, adjustment: i8) -> Result<u8, SonosError> {
        self.simulate_delay();
        self.check_for_simulated_error("adjust_volume")?;
        self.record_command(&format!("adjust_volume({})", adjustment));
        
        let mut current_volume = self.volume.lock().unwrap();
        let new_volume = (*current_volume as i16 + adjustment as i16).clamp(0, 100) as u8;
        *current_volume = new_volume;
        Ok(new_volume)
    }

    fn parse_element_u8(&self, _element: &xmltree::Element, key: &str) -> Result<u8, SonosError> {
        self.simulate_delay();
        self.check_for_simulated_error("parse_element_u8")?;
        self.record_command(&format!("parse_element_u8({})", key));
        
        // Return mock values based on key
        match key {
            "GetVolume" | "NewVolume" => Ok(*self.volume.lock().unwrap()),
            _ => Ok(0),
        }
    }
}

/// Builder for creating mock speakers with specific configurations
pub struct MockSpeakerBuilder {
    name: String,
    uuid: String,
    ip: String,
    config: MockSpeakerConfig,
}

impl MockSpeakerBuilder {
    pub fn new() -> Self {
        Self {
            name: "Mock Speaker".to_string(),
            uuid: "RINCON_MOCK_001".to_string(),
            ip: "192.168.1.100".to_string(),
            config: MockSpeakerConfig::default(),
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn uuid(mut self, uuid: &str) -> Self {
        self.uuid = uuid.to_string();
        self
    }

    pub fn ip(mut self, ip: &str) -> Self {
        self.ip = ip.to_string();
        self
    }

    pub fn unreachable(mut self) -> Self {
        self.config.simulate_unreachable = true;
        self
    }

    pub fn with_timeout(mut self) -> Self {
        self.config.simulate_timeout = true;
        self
    }

    pub fn with_parse_error(mut self) -> Self {
        self.config.simulate_parse_error = true;
        self
    }

    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.config.response_delay_ms = delay_ms;
        self
    }

    pub fn failing_commands(mut self) -> Self {
        self.config.commands_succeed = false;
        self
    }

    pub fn with_custom_error(mut self, error_message: &str) -> Self {
        self.config.custom_error_message = Some(error_message.to_string());
        self.config.commands_succeed = false;
        self
    }

    pub fn build(self) -> MockSpeaker {
        let speaker = MockSpeaker::new(&self.name, &self.uuid, &self.ip);
        speaker.set_config(self.config);
        speaker
    }
}

impl Default for MockSpeakerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock system for testing network scenarios
#[derive(Debug)]
pub struct MockSystem {
    speakers: HashMap<String, Box<dyn SpeakerTrait>>,
    zone_groups: Vec<MockZoneGroup>,
    topology: Option<crate::topology::types::Topology>,
    network_config: MockNetworkConfig,
}

/// Configuration for mock network behavior
#[derive(Debug, Clone)]
pub struct MockNetworkConfig {
    pub discovery_fails: bool,
    pub topology_retrieval_fails: bool,
    pub speaker_lookup_fails: bool,
    pub zone_group_lookup_fails: bool,
    pub discovery_delay_ms: u64,
}

impl Default for MockNetworkConfig {
    fn default() -> Self {
        Self {
            discovery_fails: false,
            topology_retrieval_fails: false,
            speaker_lookup_fails: false,
            zone_group_lookup_fails: false,
            discovery_delay_ms: 0,
        }
    }
}

/// Mock zone group for testing
#[derive(Debug, Clone)]
pub struct MockZoneGroup {
    pub coordinator: String,
    pub name: String,
    pub members: Vec<String>,
    pub config: MockSpeakerConfig,
}

impl MockZoneGroup {
    pub fn new(coordinator: &str, name: &str) -> Self {
        Self {
            coordinator: coordinator.to_string(),
            name: name.to_string(),
            members: vec![coordinator.to_string()],
            config: MockSpeakerConfig::default(),
        }
    }

    pub fn add_member(&mut self, member_uuid: &str) {
        if !self.members.contains(&member_uuid.to_string()) {
            self.members.push(member_uuid.to_string());
        }
    }

    pub fn set_config(&mut self, config: MockSpeakerConfig) {
        self.config = config;
    }

    pub fn pause(&self, system: &MockSystem) -> Result<(), SonosError> {
        if self.config.simulate_unreachable {
            return Err(SonosError::DeviceUnreachable);
        }
        
        if let Some(coordinator_speaker) = system.get_speaker_by_uuid(&self.coordinator) {
            coordinator_speaker.pause()
        } else {
            Err(SonosError::DeviceUnreachable)
        }
    }

    pub fn play(&self, system: &MockSystem) -> Result<(), SonosError> {
        if self.config.simulate_unreachable {
            return Err(SonosError::DeviceUnreachable);
        }
        
        if let Some(coordinator_speaker) = system.get_speaker_by_uuid(&self.coordinator) {
            coordinator_speaker.play()
        } else {
            Err(SonosError::DeviceUnreachable)
        }
    }
}

impl MockSystem {
    pub fn new() -> Self {
        Self {
            speakers: HashMap::new(),
            zone_groups: Vec::new(),
            topology: None,
            network_config: MockNetworkConfig::default(),
        }
    }

    pub fn set_network_config(&mut self, config: MockNetworkConfig) {
        self.network_config = config;
    }

    pub fn add_speaker(&mut self, speaker: Box<dyn SpeakerTrait>) {
        let uuid = speaker.uuid().to_string();
        self.speakers.insert(uuid, speaker);
    }

    pub fn add_zone_group(&mut self, zone_group: MockZoneGroup) {
        self.zone_groups.push(zone_group);
    }

    pub fn get_speaker_by_uuid(&self, uuid: &str) -> Option<&dyn SpeakerTrait> {
        if self.network_config.speaker_lookup_fails {
            return None;
        }
        
        self.speakers.get(uuid).map(|s| s.as_ref())
    }

    pub fn simulate_discovery(&self) -> Result<Vec<String>, SonosError> {
        if self.network_config.discovery_delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(self.network_config.discovery_delay_ms));
        }
        
        if self.network_config.discovery_fails {
            return Err(SonosError::NetworkTimeout);
        }
        
        Ok(self.speakers.keys().cloned().collect())
    }

    pub fn simulate_topology_retrieval(&self) -> Result<Option<crate::topology::types::Topology>, SonosError> {
        if self.network_config.topology_retrieval_fails {
            return Err(SonosError::ParseError("Mock topology retrieval failed".to_string()));
        }
        
        Ok(self.topology.clone())
    }

    pub fn speaker_count(&self) -> usize {
        self.speakers.len()
    }

    pub fn zone_group_count(&self) -> usize {
        self.zone_groups.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_speaker_basic_functionality() {
        let speaker = MockSpeaker::new("Test Speaker", "RINCON_TEST_001", "192.168.1.100");
        
        assert_eq!(speaker.name(), "Test Speaker");
        assert_eq!(speaker.uuid(), "RINCON_TEST_001");
        assert_eq!(speaker.ip(), "192.168.1.100");
        
        // Test successful commands
        assert!(speaker.play().is_ok());
        assert!(speaker.pause().is_ok());
        assert_eq!(speaker.get_volume().unwrap(), 50);
        assert_eq!(speaker.set_volume(75).unwrap(), 75);
        assert_eq!(speaker.get_volume().unwrap(), 75);
    }

    #[test]
    fn test_mock_speaker_command_history() {
        let speaker = MockSpeaker::new("Test Speaker", "RINCON_TEST_001", "192.168.1.100");
        
        assert!(speaker.get_command_history().is_empty());
        
        let _ = speaker.play();
        let _ = speaker.pause();
        let _ = speaker.set_volume(60);
        
        let history = speaker.get_command_history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], "play");
        assert_eq!(history[1], "pause");
        assert_eq!(history[2], "set_volume(60)");
        
        speaker.clear_command_history();
        assert!(speaker.get_command_history().is_empty());
    }

    #[test]
    fn test_mock_speaker_unreachable_simulation() {
        let speaker = MockSpeakerBuilder::new()
            .name("Unreachable Speaker")
            .uuid("RINCON_UNREACHABLE_001")
            .unreachable()
            .build();
        
        assert!(speaker.play().is_err());
        assert!(speaker.pause().is_err());
        assert!(speaker.get_volume().is_err());
        
        match speaker.play().unwrap_err() {
            SonosError::DeviceUnreachable => {
                // Expected
            }
            _ => panic!("Expected DeviceUnreachable error"),
        }
    }

    #[test]
    fn test_mock_speaker_timeout_simulation() {
        let speaker = MockSpeakerBuilder::new()
            .name("Timeout Speaker")
            .uuid("RINCON_TIMEOUT_001")
            .with_timeout()
            .build();
        
        match speaker.play().unwrap_err() {
            SonosError::NetworkTimeout => {
                // Expected
            }
            _ => panic!("Expected NetworkTimeout error"),
        }
    }

    #[test]
    fn test_mock_speaker_parse_error_simulation() {
        let speaker = MockSpeakerBuilder::new()
            .name("Parse Error Speaker")
            .uuid("RINCON_PARSE_ERROR_001")
            .with_parse_error()
            .build();
        
        match speaker.play().unwrap_err() {
            SonosError::ParseError(msg) => {
                assert!(msg.contains("Mock parse error"));
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_mock_speaker_custom_error() {
        let speaker = MockSpeakerBuilder::new()
            .name("Custom Error Speaker")
            .uuid("RINCON_CUSTOM_ERROR_001")
            .with_custom_error("Custom test error message")
            .build();
        
        match speaker.play().unwrap_err() {
            SonosError::ParseError(msg) => {
                assert_eq!(msg, "Custom test error message");
            }
            _ => panic!("Expected custom ParseError"),
        }
    }

    #[test]
    fn test_mock_speaker_delay_simulation() {
        let speaker = MockSpeakerBuilder::new()
            .name("Delayed Speaker")
            .uuid("RINCON_DELAYED_001")
            .with_delay(10) // 10ms delay
            .build();
        
        let start = std::time::Instant::now();
        let _ = speaker.play();
        let elapsed = start.elapsed();
        
        assert!(elapsed.as_millis() >= 10);
    }

    #[test]
    fn test_mock_speaker_volume_adjustment() {
        let speaker = MockSpeaker::new("Volume Test", "RINCON_VOLUME_001", "192.168.1.100");
        
        // Test volume adjustment
        assert_eq!(speaker.adjust_volume(10).unwrap(), 60); // 50 + 10
        assert_eq!(speaker.adjust_volume(-20).unwrap(), 40); // 60 - 20
        
        // Test volume clamping
        assert_eq!(speaker.adjust_volume(100).unwrap(), 100); // Clamp to max
        assert_eq!(speaker.adjust_volume(-100).unwrap(), 0); // Clamp to min
    }

    #[test]
    fn test_mock_system_basic_functionality() {
        let mut system = MockSystem::new();
        
        let speaker1 = Box::new(MockSpeaker::new("Speaker 1", "RINCON_001", "192.168.1.100"));
        let speaker2 = Box::new(MockSpeaker::new("Speaker 2", "RINCON_002", "192.168.1.101"));
        
        system.add_speaker(speaker1);
        system.add_speaker(speaker2);
        
        assert_eq!(system.speaker_count(), 2);
        
        let found_speaker = system.get_speaker_by_uuid("RINCON_001");
        assert!(found_speaker.is_some());
        assert_eq!(found_speaker.unwrap().name(), "Speaker 1");
        
        let not_found = system.get_speaker_by_uuid("RINCON_999");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_mock_system_network_failures() {
        let mut system = MockSystem::new();
        
        let config = MockNetworkConfig {
            discovery_fails: true,
            speaker_lookup_fails: true,
            ..Default::default()
        };
        system.set_network_config(config);
        
        // Discovery should fail
        assert!(system.simulate_discovery().is_err());
        
        // Speaker lookup should fail even if speaker exists
        let speaker = Box::new(MockSpeaker::new("Test", "RINCON_001", "192.168.1.100"));
        system.add_speaker(speaker);
        assert!(system.get_speaker_by_uuid("RINCON_001").is_none());
    }

    #[test]
    fn test_mock_zone_group_commands() {
        let mut system = MockSystem::new();
        
        let speaker = Box::new(MockSpeaker::new("Coordinator", "RINCON_COORD", "192.168.1.100"));
        system.add_speaker(speaker);
        
        let zone_group = MockZoneGroup::new("RINCON_COORD", "Test Group");
        
        // Commands should succeed
        assert!(zone_group.play(&system).is_ok());
        assert!(zone_group.pause(&system).is_ok());
        
        // Test with unreachable configuration
        let mut failing_group = MockZoneGroup::new("RINCON_COORD", "Failing Group");
        failing_group.set_config(MockSpeakerConfig {
            simulate_unreachable: true,
            ..Default::default()
        });
        
        assert!(failing_group.play(&system).is_err());
        assert!(failing_group.pause(&system).is_err());
    }
}