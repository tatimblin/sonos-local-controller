use super::subscription::{ServiceSubscription, SubscriptionError, SubscriptionResult};
use super::types::{ServiceType, SubscriptionConfig, SubscriptionId, SubscriptionScope};
use crate::models::{GroupId, Speaker, SpeakerId, StateChange};
use crate::service::zone_group_topology::topology_changes::TopologyChanges;
use crate::service::zone_group_topology::topology_snapshot::TopologySnapshot;
use crate::service::zone_group_topology::{self, topology_changes, topology_snapshot};
use crate::service::zone_group_topology::parser::ZoneGroups;

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::SystemTime;

/// ZoneGroupTopology service subscription implementation
///
/// This struct handles UPnP subscriptions to the ZoneGroupTopology service on Sonos devices,
/// which provides events for zone group changes, speaker grouping/ungrouping, and topology
/// updates across all Sonos devices in the network. Unlike per-speaker services, this is a
/// network-wide service that only requires one subscription per network.
pub struct ZoneGroupTopologySubscription {
    /// Representative speaker for this network (used for subscription endpoint)
    representative_speaker: Speaker,
    /// Current subscription ID (None if not subscribed)
    subscription_id: Option<SubscriptionId>,
    /// UPnP SID (Subscription ID) returned by the device
    upnp_sid: Option<String>,
    /// URL where the device should send event notifications
    callback_url: String,
    /// Configuration for this subscription
    config: SubscriptionConfig,
    /// Whether the subscription is currently active
    active: bool,
    /// Timestamp of the last successful renewal
    last_renewal: Option<SystemTime>,
    /// Last known zone group state for change detection (using Mutex for thread-safe interior mutability)
    last_zone_topology: Mutex<Option<zone_group_topology::parser::ZoneGroups>>,
}

impl ZoneGroupTopologySubscription {
  /// Create a new ZoneGroupTopology subscription
  ///
  /// # Arguments
  /// * `representative_speaker` - The speaker to use for the subscription endpoint
  /// * `callback_url` - URL where the device should send event notifications
  /// * `config` - Configuration for this subscription
  pub fn new(
      representative_speaker: Speaker,
      callback_url: String,
      config: SubscriptionConfig,
  ) -> SubscriptionResult<Self> {
      Ok(Self {
          representative_speaker,
          subscription_id: None,
          upnp_sid: None,
          callback_url,
          config,
          active: false,
          last_renewal: None,
          last_zone_topology: Mutex::new(None),
      })
  }

  /// Get the device URL for the representative speaker
  fn device_url(&self) -> String {
      format!(
          "http://{}:{}",
          self.representative_speaker.ip_address, self.representative_speaker.port
      )
  }

  /// Send a UPnP SUBSCRIBE request to establish the subscription
  fn send_subscribe_request(&self) -> SubscriptionResult<String> {
      let device_url = self.device_url();
      let event_sub_url = ServiceType::ZoneGroupTopology.event_sub_url();
      let full_url = format!("{}{}", device_url, event_sub_url);

      println!(
          "📡 Sending ZoneGroupTopology SUBSCRIBE request to: {}",
          full_url
      );
      println!("   Callback URL: {}", self.callback_url);

      // Create HTTP client for subscription requests with timeout
      let client = reqwest::blocking::Client::builder()
          .timeout(std::time::Duration::from_secs(10))
          .build()
          .map_err(|e| SubscriptionError::NetworkError(e.to_string()))?;

      println!("🔄 Making HTTP SUBSCRIBE request...");
      let response = client
          .request(
              reqwest::Method::from_bytes(b"SUBSCRIBE").unwrap(),
              &full_url,
          )
          .header(
              "HOST",
              format!(
                  "{}:{}",
                  self.representative_speaker.ip_address, self.representative_speaker.port
              ),
          )
          .header("CALLBACK", format!("<{}>", self.callback_url))
          .header("NT", "upnp:event")
          .header("TIMEOUT", format!("Second-{}", self.config.timeout_seconds))
          .send()
          .map_err(|e| {
              println!("❌ HTTP request failed: {}", e);
              SubscriptionError::NetworkError(e.to_string())
          })?;

      if !response.status().is_success() {
          return match response.status().as_u16() {
              503 => {
                  // Don't print error message here - let the caller handle satellite speaker detection
                  Err(SubscriptionError::SatelliteSpeaker)
              }
              _ => {
                  let error_msg = format!(
                      "HTTP {} - {}",
                      response.status(),
                      response.status().canonical_reason().unwrap_or("Unknown")
                  );
                  Err(SubscriptionError::SubscriptionFailed(error_msg))
              }
          };
      }

      // Extract SID from response headers
      let sid = response
          .headers()
          .get("SID")
          .and_then(|v| v.to_str().ok())
          .ok_or_else(|| {
              SubscriptionError::SubscriptionFailed("No SID in response".to_string())
          })?;

      Ok(sid.to_string())
  }

  /// Send a UPnP UNSUBSCRIBE request to terminate the subscription
  fn send_unsubscribe_request(&self, sid: &str) -> SubscriptionResult<()> {
      let device_url = self.device_url();
      let event_sub_url = ServiceType::ZoneGroupTopology.event_sub_url();
      let full_url = format!("{}{}", device_url, event_sub_url);

      let client = reqwest::blocking::Client::builder()
          .timeout(std::time::Duration::from_secs(10))
          .build()
          .map_err(|e| SubscriptionError::NetworkError(e.to_string()))?;

      let response = client
          .request(
              reqwest::Method::from_bytes(b"UNSUBSCRIBE").unwrap(),
              &full_url,
          )
          .header(
              "HOST",
              format!(
                  "{}:{}",
                  self.representative_speaker.ip_address, self.representative_speaker.port
              ),
          )
          .header("SID", sid)
          .send()
          .map_err(|e| SubscriptionError::NetworkError(e.to_string()))?;

      if !response.status().is_success() {
          return Err(SubscriptionError::SubscriptionFailed(format!(
              "UNSUBSCRIBE failed: HTTP {}",
              response.status()
          )));
      }

      Ok(())
  }

  /// Send a subscription renewal request
  fn send_renewal_request(&self, sid: &str) -> SubscriptionResult<()> {
      let device_url = self.device_url();
      let event_sub_url = ServiceType::ZoneGroupTopology.event_sub_url();
      let full_url = format!("{}{}", device_url, event_sub_url);

      let client = reqwest::blocking::Client::builder()
          .timeout(std::time::Duration::from_secs(10))
          .build()
          .map_err(|e| SubscriptionError::NetworkError(e.to_string()))?;

      let response = client
          .request(
              reqwest::Method::from_bytes(b"SUBSCRIBE").unwrap(),
              &full_url,
          )
          .header(
              "HOST",
              format!(
                  "{}:{}",
                  self.representative_speaker.ip_address, self.representative_speaker.port
              ),
          )
          .header("SID", sid)
          .header("TIMEOUT", format!("Second-{}", self.config.timeout_seconds))
          .send()
          .map_err(|e| SubscriptionError::NetworkError(e.to_string()))?;

      if !response.status().is_success() {
          return Err(SubscriptionError::SubscriptionFailed(format!(
              "Renewal failed: HTTP {}",
              response.status()
          )));
      }

      Ok(())
  }

  /// Detect changes between old and new zone group states
  fn detect_topology_changes(&self, new_topology: &ZoneGroups) -> Vec<StateChange> {
    let mut last_topology = self.last_zone_topology.lock().unwrap();

    let changes = match last_topology.as_ref() {
      None => topology_changes::TopologyChanges::new(),
      Some(old_topology) => {
        self.compute_changes(old_topology, new_topology)
      }
    };

    *last_topology = Some(new_topology.clone());

    let changes = changes.into_vec();
    changes
  }

  fn compute_changes(&self, old_topology: &ZoneGroups, new_topology: &ZoneGroups) -> topology_changes::TopologyChanges {
    let old_snapshot = topology_snapshot::TopologySnapshot::from_parser(old_topology);
    let new_snapshot = topology_snapshot::TopologySnapshot::from_parser(new_topology);

    let mut changes = TopologyChanges::new();

    self.detect_group_formations(&old_snapshot, &new_snapshot, &mut changes);
    self.detect_group_dissolutions(&old_snapshot, &new_snapshot, &mut changes);
    self.detect_membership_changes(&old_snapshot, &new_snapshot, &mut changes);
    self.detect_coordinator_changes(&old_snapshot, &new_snapshot, &mut changes);

    changes
  }

  fn detect_group_formations(
    &self,
    old: &TopologySnapshot,
    new: &TopologySnapshot,
    changes: &mut TopologyChanges
  ) {
    for (group_id, (coordinator_id, members)) in &new.groups {
      if !old.groups.contains_key(group_id) {
        changes.add(StateChange::GroupFormed {
          group_id: *group_id,
          coordinator_id: *coordinator_id,
          initial_members: members.iter().copied().collect(),
        });
      }
    }
  }

  fn detect_group_dissolutions(
    &self,
    old: &TopologySnapshot,
    new: &TopologySnapshot,
    changes: &mut TopologyChanges,
  ) {
    for (group_id, (coordinator_id, members)) in &old.groups {
      if !new.groups.contains_key(group_id) {
        changes.add(StateChange::GroupDissolved {
          group_id: *group_id,
          former_coordinator: *coordinator_id,
          former_members: members.iter().copied().collect()
        })
      }
    }
  }

  fn detect_membership_changes(
    &self,
    old: &TopologySnapshot,
    new: &TopologySnapshot,
    changes: &mut TopologyChanges,
  ) {
    let old_speaker_map = old.get_speaker_group_map();
    let new_speaker_map = new.get_speaker_group_map();

    let all_speakers: HashSet<_> = old_speaker_map.keys()
      .chain(new_speaker_map.keys())
      .copied()
      .collect();

    for speaker_id in all_speakers {
      let old_group = old_speaker_map.get(&speaker_id);
      let new_group = new_speaker_map.get(&speaker_id);

      match (old_group, new_group) {
        (Some(old_gid), Some(new_gid)) if old_gid != new_gid => {
          // Speaker moved between groups
          self.handle_speaker_left(*old_gid, speaker_id, changes);
          self.handle_speaker_joined(*new_gid, speaker_id, new, changes);
        }
        (Some(old_gid), None) => {
          // Speaker left group (became solo)
          self.handle_speaker_left(*old_gid, speaker_id, changes);
        }
        (None, Some(new_gid)) => {
          // Speaker joined group (was solo)
          self.handle_speaker_joined(*new_gid, speaker_id, new, changes);
        }
        _ => {
          // No change (same group or still solo)
        }
      }
    }
  }

  fn handle_speaker_left(
    &self,
    group_id: GroupId,
    speaker_id: SpeakerId,
    changes: &mut TopologyChanges,
  ) {
    changes.add(StateChange::SpeakerLeftGroup {
      speaker_id,
      former_group_id: group_id,
    });
  }

  fn handle_speaker_joined(
    &self,
    group_id: GroupId,
    speaker_id: SpeakerId,
    snapshot: &TopologySnapshot,
    changes: &mut TopologyChanges,
  ) {
    let coordinator_id = snapshot.groups
      .get(&group_id)
      .map(|(coordinator, _)| *coordinator)
      .unwrap_or(speaker_id);

    changes.add(StateChange::SpeakerJoinedGroup {
      speaker_id,
      group_id,
      coordinator_id,
    });
  }

  /// Detect coordinator changes within existing groups
  fn detect_coordinator_changes(
    &self,
    old: &TopologySnapshot,
    new: &TopologySnapshot,
    changes: &mut TopologyChanges,
  ) {
    for (group_id, (new_coordinator, _)) in &new.groups {
      if let Some((old_coordinator, _)) = old.groups.get(group_id) {
        if old_coordinator != new_coordinator {
          changes.add(StateChange::CoordinatorChanged {
            group_id: *group_id,
            old_coordinator: *old_coordinator,
            new_coordinator: *new_coordinator,
          });
        }
      }
    }
  }
}

impl ServiceSubscription for ZoneGroupTopologySubscription {
    fn service_type(&self) -> ServiceType {
        ServiceType::ZoneGroupTopology
    }

    fn subscription_scope(&self) -> SubscriptionScope {
        SubscriptionScope::NetworkWide
    }

    fn speaker_id(&self) -> SpeakerId {
        // Return the representative speaker's ID
        self.representative_speaker.id
    }

    fn subscribe(&mut self) -> SubscriptionResult<SubscriptionId> {
        // Send SUBSCRIBE request
        let upnp_sid = self.send_subscribe_request()?;

        // Create subscription ID and update state
        let subscription_id = SubscriptionId::new();
        self.subscription_id = Some(subscription_id);
        self.upnp_sid = Some(upnp_sid);
        self.active = true;
        self.last_renewal = Some(SystemTime::now());

        println!(
            "✅ ZoneGroupTopology subscription established with ID: {}",
            subscription_id
        );
        Ok(subscription_id)
    }

    fn unsubscribe(&mut self) -> SubscriptionResult<()> {
        if let Some(upnp_sid) = &self.upnp_sid {
            self.send_unsubscribe_request(upnp_sid)?;
        }

        self.subscription_id = None;
        self.upnp_sid = None;
        self.active = false;
        self.last_renewal = None;
        *self.last_zone_topology.lock().unwrap() = None;

        println!("✅ ZoneGroupTopology subscription terminated");
        Ok(())
    }

    fn renew(&mut self) -> SubscriptionResult<()> {
        if !self.active {
            return Err(SubscriptionError::SubscriptionExpired);
        }

        if let Some(upnp_sid) = &self.upnp_sid {
            self.send_renewal_request(upnp_sid)?;
            self.last_renewal = Some(SystemTime::now());
            println!("✅ ZoneGroupTopology subscription renewed");
            Ok(())
        } else {
            Err(SubscriptionError::SubscriptionExpired)
        }
    }

    fn parse_event(&self, event_xml: &str) -> SubscriptionResult<Vec<StateChange>> {
        let mut changes = Vec::new();

        match zone_group_topology::parser::ZoneGroupTopologyParser::from_xml(event_xml) {
            Ok(parser) => {
                println!("🔍 Parsing ZoneGroupTopology event...");
                
                // Detect changes and generate appropriate StateChange events directly from parser
                if let Some(zone_group_property) = parser.zone_group_state() {
                    if let Some(zone_group_state) = &zone_group_property.zone_group_state {
                        changes = self.detect_topology_changes(&zone_group_state.zone_groups);
                    }
                }
                
                println!("✅ Generated {} state changes from ZoneGroupTopology event", changes.len());
            }
            Err(e) => {
                println!("❌ Failed to parse ZoneGroupTopology XML: {}", e);
                // Log the error but don't fail completely - return subscription error
                changes.push(StateChange::SubscriptionError {
                    speaker_id: self.representative_speaker.id,
                    service: ServiceType::ZoneGroupTopology,
                    error: format!("XML parsing failed: {}", e),
                });
            }
        }

        Ok(changes)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn last_renewal(&self) -> Option<SystemTime> {
        self.last_renewal
    }

    fn subscription_id(&self) -> Option<SubscriptionId> {
        self.subscription_id
    }

    fn get_config(&self) -> &SubscriptionConfig {
        &self.config
    }

    fn callback_url(&self) -> &str {
        &self.callback_url
    }

    fn on_subscription_state_changed(&mut self, active: bool) -> SubscriptionResult<()> {
        self.active = active;
        if !active {
            self.subscription_id = None;
            self.upnp_sid = None;
            self.last_renewal = None;
            *self.last_zone_topology.lock().unwrap() = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Speaker;

    fn create_test_speaker(id_suffix: &str, ip: &str) -> Speaker {
        Speaker {
            id: SpeakerId::from_udn(&format!("uuid:RINCON_{}::1", id_suffix)),
            udn: format!("uuid:RINCON_{}::1", id_suffix),
            name: format!("Test Speaker {}", id_suffix),
            room_name: format!("Test Room {}", id_suffix),
            ip_address: ip.to_string(),
            port: 1400,
            model_name: "Test Model".to_string(),
            satellites: vec![],
        }
    }

    #[test]
    fn test_zone_group_topology_subscription_creation() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let callback_url = "http://localhost:8080/callback/test".to_string();
        let config = SubscriptionConfig::default();

        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker.clone(),
            callback_url.clone(),
            config,
        );

        assert!(subscription.is_ok());

        let sub = subscription.unwrap();
        assert_eq!(sub.service_type(), ServiceType::ZoneGroupTopology);
        assert_eq!(sub.subscription_scope(), SubscriptionScope::NetworkWide);
        assert_eq!(sub.speaker_id(), representative_speaker.id);
        assert_eq!(sub.callback_url(), &callback_url);
        assert!(!sub.is_active());
        assert!(sub.subscription_id().is_none());
    }
}
