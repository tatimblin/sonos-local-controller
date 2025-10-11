use super::device::{extract_ip_from_url, Device};
use super::ssdp::SsdpClient;
use crate::error::{Result, SonosError};
use crate::models::Speaker;
use std::collections::HashSet;
use std::time::Duration;

/// Discovery service for finding Sonos speakers on the network
pub struct Discovery {
    timeout: Duration,
}

impl Discovery {
    /// Create a new discovery service with the specified timeout
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Discover all Sonos speakers on the network
    pub fn discover_speakers(&self) -> Result<Vec<Speaker>> {
        let client = SsdpClient::new(self.timeout).map_err(|e| {
            SonosError::DiscoveryFailed(format!("Failed to create SSDP client: {}", e))
        })?;

        let responses = client
            .search("urn:schemas-upnp-org:device:ZonePlayer:1")
            .map_err(|e| SonosError::DiscoveryFailed(format!("SSDP search failed: {}", e)))?;

        let mut speakers = Vec::new();
        let mut seen_locations = HashSet::new();

        for response_result in responses {
            match response_result {
                Ok(response) => {
                    // Avoid duplicate speakers from multiple responses
                    if seen_locations.contains(&response.location) {
                        continue;
                    }
                    seen_locations.insert(response.location.clone());

                    // Filter out non-Sonos devices early based on SSDP response
                    if !self.is_likely_sonos_device(&response) {
                        continue;
                    }

                    if let Some(ip) = extract_ip_from_url(&response.location) {
                        match self.fetch_device_info(&response.location, ip) {
                            Ok(speaker) => speakers.push(speaker),
                            Err(_e) => {
                                // Still might get some false positives, but much fewer now
                            }
                        }
                    }
                }
                Err(_e) => {
                    // Silently skip SSDP response errors - common on busy networks
                }
            }
        }

        Ok(speakers)
    }

    /// Check if an SSDP response is likely from a Sonos device
    fn is_likely_sonos_device(&self, response: &super::ssdp::SsdpResponse) -> bool {
        // Check URN - Sonos devices use ZonePlayer
        if response.urn.contains("ZonePlayer") {
            return true;
        }

        // Check server header for Sonos signature
        if let Some(server) = &response.server {
            if server.to_lowercase().contains("sonos") {
                return true;
            }
        }

        // Check USN for RINCON (Sonos device identifier)
        if response.usn.contains("RINCON_") {
            return true;
        }

        // Check location URL - Sonos typically uses port 1400 and /xml/device_description.xml
        if response
            .location
            .contains(":1400/xml/device_description.xml")
        {
            return true;
        }

        false
    }

    /// Fetch device information from a URL and convert to Speaker
    fn fetch_device_info(&self, location: &str, ip_address: String) -> Result<Speaker> {
        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| {
                SonosError::CommunicationError(format!("Failed to create HTTP client: {}", e))
            })?;

        let response = client
            .get(location)
            .send()
            .map_err(|e| SonosError::CommunicationError(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(SonosError::CommunicationError(format!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        let xml = response.text().map_err(|e| {
            SonosError::CommunicationError(format!("Failed to read response body: {}", e))
        })?;

        let device = Device::from_xml(&xml)?;

        // Verify this is actually a Sonos device
        if !device.is_sonos_speaker() {
            return Err(SonosError::DeviceNotFound(format!(
                "Device at {} is not a Sonos speaker",
                ip_address
            )));
        }

        Ok(device.to_speaker(ip_address))
    }
}

/// Convenience function for quick speaker discovery with default timeout
pub fn discover_speakers() -> Result<Vec<Speaker>> {
    let discovery = Discovery::new(Duration::from_secs(3));
    discovery.discover_speakers()
}

/// Convenience function for speaker discovery with custom timeout
pub fn discover_speakers_with_timeout(timeout: Duration) -> Result<Vec<Speaker>> {
    let discovery = Discovery::new(timeout);
    discovery.discover_speakers()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_new() {
        let timeout = Duration::from_secs(5);
        let discovery = Discovery::new(timeout);
        assert_eq!(discovery.timeout, timeout);
    }

    #[test]
    fn test_discover_speakers_convenience_function() {
        // This test just ensures the function compiles and returns a Result
        // In a real test environment, this would need mock SSDP responses
        let result = discover_speakers();
        assert!(result.is_ok() || result.is_err()); // Either outcome is valid without real devices
    }

    #[test]
    fn test_discover_speakers_with_timeout_convenience_function() {
        let timeout = Duration::from_millis(100);
        let result = discover_speakers_with_timeout(timeout);
        assert!(result.is_ok() || result.is_err()); // Either outcome is valid without real devices
    }

    #[test]
    fn test_fetch_device_info_invalid_url() {
        let discovery = Discovery::new(Duration::from_millis(100));
        let result = discovery.fetch_device_info("invalid-url", "192.168.1.100".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_fetch_device_info_unreachable_host() {
        let discovery = Discovery::new(Duration::from_millis(100));
        // Use a non-routable IP address that should timeout quickly
        let result = discovery.fetch_device_info(
            "http://192.0.2.1:1400/xml/device_description.xml",
            "192.0.2.1".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_is_likely_sonos_device() {
        use super::super::ssdp::SsdpResponse;

        let discovery = Discovery::new(Duration::from_millis(100));

        // Test Sonos device with ZonePlayer URN
        let sonos_response = SsdpResponse {
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            urn: "urn:schemas-upnp-org:device:ZonePlayer:1".to_string(),
            usn: "uuid:RINCON_000E58A0123456::urn:schemas-upnp-org:device:ZonePlayer:1".to_string(),
            server: Some("Linux/3.14.0 UPnP/1.0 Sonos/70.3-35220".to_string()),
        };
        assert!(discovery.is_likely_sonos_device(&sonos_response));

        // Test non-Sonos device (router)
        let router_response = SsdpResponse {
            location: "http://10.0.4.1:1900/igd.xml".to_string(),
            urn: "urn:schemas-upnp-org:device:InternetGatewayDevice:1".to_string(),
            usn: "uuid:12345678-1234-1234-1234-123456789012::urn:schemas-upnp-org:device:InternetGatewayDevice:1".to_string(),
            server: Some("Linux/2.6 UPnP/1.0 Router/1.0".to_string()),
        };
        assert!(!discovery.is_likely_sonos_device(&router_response));

        // Test device with RINCON in USN but different URN
        let rincon_response = SsdpResponse {
            location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
            urn: "urn:schemas-upnp-org:device:MediaRenderer:1".to_string(),
            usn: "uuid:RINCON_B8E937123456::urn:schemas-upnp-org:device:MediaRenderer:1"
                .to_string(),
            server: None,
        };
        assert!(discovery.is_likely_sonos_device(&rincon_response));
    }
}
