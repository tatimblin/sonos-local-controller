//! Network client for Sonos topology retrieval
//!
//! This module handles all network communication for retrieving topology information
//! from Sonos speakers, including debug logging and response handling.

use log::{debug, error, info};
use crate::{model::Action, Client, SonosError};
use crate::topology::types::Topology;
use crate::topology::utils::element_to_str;
use std::fs::OpenOptions;
use std::io::Write;
use xmltree::Element;

/// Client for retrieving topology information from Sonos speakers
pub struct TopologyClient {
    /// Underlying HTTP client for communication with Sonos devices
    client: Client,
}

impl TopologyClient {
    /// Creates a new topology client with default settings
    pub fn new() -> Self {
        Self {
            client: Client::default(),
        }
    }

    /// Creates a new topology client with a custom HTTP client
    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    /// Retrieves topology information from a Sonos speaker at the given IP address
    ///
    /// # Arguments
    /// * `ip` - IP address of a Sonos speaker to query
    ///
    /// # Returns
    /// * `Ok(Topology)` - Complete topology information for the Sonos system
    /// * `Err(SonosError)` - If the request fails or parsing fails
    pub fn get_topology(&self, ip: &str) -> Result<Topology, SonosError> {
        info!("Starting topology retrieval from IP: {}", ip);
        
        let payload = "<InstanceID>0</InstanceID>";
        debug!("Using payload: {}", payload);

        info!("Sending GetZoneGroupState action to {}...", ip);
        let response = self.client.send_action(ip, Action::GetZoneGroupState, payload)
            .map_err(|e| {
                error!("Failed to send action to {}: {:?}", ip, e);
                e
            })?;

        info!("Successfully received response from {}", ip);
        
        // Log raw response for debugging (optional, can be disabled in production)
        if cfg!(debug_assertions) {
            self.log_raw_response(&response);
        }

        let response_str = element_to_str(&response);
        debug!("Response XML length: {} characters", response_str.len());

        info!("Parsing XML response...");
        let topology = Topology::from_xml(&response_str)
            .map_err(|e| {
                error!("Failed to parse XML: {:?}", e);
                e
            })?;

        info!("Successfully parsed topology with {} zone groups", topology.zone_groups.len());
        Ok(topology)
    }

    /// Logs the raw XML response for debugging purposes
    fn log_raw_response(&self, response: &Element) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("../log.txt") {
            let response_str = element_to_str(response);
            let _ = file.write_all(response_str.as_bytes());
            let _ = file.write_all(b"\n--- END RESPONSE ---\n");
        }
    }
}

impl Default for TopologyClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Retrieves topology information from a Sonos speaker at the given IP address
///
/// This is a convenience function that creates a default TopologyClient and
/// retrieves the topology information.
///
/// # Arguments
/// * `ip` - IP address of a Sonos speaker to query
///
/// # Returns
/// * `Ok(Topology)` - Complete topology information for the Sonos system
/// * `Err(SonosError)` - If the request fails or parsing fails
pub fn get_topology_from_ip(ip: &str) -> Result<Topology, SonosError> {
    let client = TopologyClient::new();
    client.get_topology(ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topology_client_new() {
        let _client = TopologyClient::new();
        // Just verify it can be created without panicking
        assert!(true);
    }

    #[test]
    fn test_topology_client_default() {
        let _client = TopologyClient::default();
        // Just verify it can be created without panicking
        assert!(true);
    }

    #[test]
    fn test_topology_client_with_client() {
        let http_client = Client::default();
        let _topology_client = TopologyClient::with_client(http_client);
        // Just verify it can be created without panicking
        assert!(true);
    }

    // Note: Testing actual network communication would require a mock server
    // or integration tests with real Sonos devices. For unit tests, we focus
    // on testing the structure and basic functionality.

    #[test]
    fn test_get_topology_from_ip_function_signature() {
        // This test verifies that the public function exists and has the correct signature
        // We test with an invalid IP that will definitely fail quickly
        let result = get_topology_from_ip("0.0.0.0");
        // Should return an error (not panic) for invalid IP
        assert!(result.is_err());
    }

    #[test]
    fn test_topology_client_get_topology_method_signature() {
        let client = TopologyClient::new();
        
        // Test that the method exists and has the correct signature
        // We test with an invalid IP that will definitely fail quickly
        let result = client.get_topology("0.0.0.0");
        // Should return an error (not panic) for invalid IP
        assert!(result.is_err());
    }

    // Integration tests for actual network communication should be in a separate
    // integration test file or marked with #[ignore] and run only when needed
    
    #[test]
    #[ignore] // Ignored by default as it requires network access
    fn test_topology_client_integration_invalid_ip() {
        let client = TopologyClient::new();
        let result = client.get_topology("192.168.255.255"); // Likely invalid IP
        
        // Should return an error for invalid/unreachable IP
        assert!(result.is_err());
    }

    #[test]
    #[ignore] // Ignored by default as it requires network access
    fn test_get_topology_from_ip_integration_invalid_ip() {
        let result = get_topology_from_ip("192.168.255.255"); // Likely invalid IP
        
        // Should return an error for invalid/unreachable IP
        assert!(result.is_err());
    }
}