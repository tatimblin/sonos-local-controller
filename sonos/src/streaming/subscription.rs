use std::time::SystemTime;
use crate::models::{SpeakerId, StateChange};
use super::types::{ServiceType, SubscriptionId, SubscriptionConfig};

/// Error types for subscription operations
#[derive(Debug, thiserror::Error)]
pub enum SubscriptionError {
    #[error("Failed to establish subscription: {0}")]
    SubscriptionFailed(String),
    
    #[error("Subscription expired or was rejected by device")]
    SubscriptionExpired,
    
    #[error("Failed to parse event notification: {0}")]
    EventParseError(String),
    
    #[error("Callback server error: {0}")]
    CallbackServerError(String),
    
    #[error("Network communication error: {0}")]
    NetworkError(String),
    
    #[error("Service not supported by device: {service:?}")]
    ServiceNotSupported { service: ServiceType },

    #[error("Invalid subscription configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Subscription not found: {subscription_id}")]
    SubscriptionNotFound { subscription_id: SubscriptionId },

    #[error("HTTP request failed: {0}")]
    HttpError(String),

    #[error("XML parsing failed: {0}")]
    XmlParseError(String),

    #[error("Timeout occurred during operation: {0}")]
    Timeout(String),
}

impl From<reqwest::Error> for SubscriptionError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            SubscriptionError::Timeout(err.to_string())
        } else {
            SubscriptionError::NetworkError(err.to_string())
        }
    }
}

impl From<quick_xml::Error> for SubscriptionError {
    fn from(err: quick_xml::Error) -> Self {
        SubscriptionError::XmlParseError(err.to_string())
    }
}

/// Result type for subscription operations
pub type SubscriptionResult<T> = Result<T, SubscriptionError>;

/// Abstract trait for service-specific UPnP subscriptions
/// 
/// This trait defines the lifecycle and behavior of a UPnP service subscription.
/// Implementations handle service-specific SOAP operations, event parsing, and
/// subscription management for different UPnP services like AVTransport or RenderingControl.
pub trait ServiceSubscription: Send + Sync {
    /// Get the service type this subscription handles
    fn service_type(&self) -> ServiceType;

    /// Get the speaker ID this subscription is associated with
    fn speaker_id(&self) -> SpeakerId;

    /// Establish a new UPnP subscription with the device
    /// 
    /// This method sends a SUBSCRIBE request to the device's event subscription URL
    /// and returns the subscription ID if successful.
    fn subscribe(&mut self) -> SubscriptionResult<SubscriptionId>;

    /// Unsubscribe from the UPnP service
    /// 
    /// This method sends an UNSUBSCRIBE request to properly clean up the subscription
    /// on the device side.
    fn unsubscribe(&mut self) -> SubscriptionResult<()>;

    /// Renew an existing subscription before it expires
    /// 
    /// This method extends the subscription timeout by sending a renewal request
    /// to the device.
    fn renew(&mut self) -> SubscriptionResult<()>;

    /// Parse a UPnP event notification into StateChange events
    /// 
    /// This method takes the raw XML from a UPnP event notification and converts
    /// it into one or more StateChange events that can be consumed by the application.
    fn parse_event(&self, event_xml: &str) -> SubscriptionResult<Vec<StateChange>>;

    /// Check if the subscription is currently active
    /// 
    /// Returns true if the subscription has been established and has not expired
    /// or been unsubscribed.
    fn is_active(&self) -> bool;

    /// Get the timestamp of the last successful renewal
    /// 
    /// Returns None if the subscription has never been renewed or established.
    fn last_renewal(&self) -> Option<SystemTime>;

    /// Get the current subscription ID
    /// 
    /// Returns None if no subscription is currently active.
    fn subscription_id(&self) -> Option<SubscriptionId>;

    /// Check if the subscription needs renewal
    /// 
    /// Returns true if the subscription is active but approaching expiry
    /// based on the renewal threshold in the configuration.
    fn needs_renewal(&self) -> bool {
        if let Some(last_renewal) = self.last_renewal() {
            if let Ok(elapsed) = last_renewal.elapsed() {
                let config = self.get_config();
                let renewal_time = std::time::Duration::from_secs(config.timeout_seconds as u64)
                    .saturating_sub(config.renewal_threshold);
                return elapsed >= renewal_time;
            }
        }
        false
    }

    /// Get the subscription configuration
    fn get_config(&self) -> &SubscriptionConfig;

    /// Get the callback URL for this subscription
    /// 
    /// This URL is where the device will send event notifications.
    fn callback_url(&self) -> &str;

    /// Handle subscription lifecycle events
    /// 
    /// This method is called when subscription state changes occur,
    /// allowing implementations to perform cleanup or state updates.
    fn on_subscription_state_changed(&mut self, active: bool) -> SubscriptionResult<()> {
        // Default implementation does nothing
        let _ = active;
        Ok(())
    }
}

/// Helper trait for creating service subscriptions
pub trait ServiceSubscriptionFactory {
    /// Create a new service subscription for the given speaker and service type
    fn create_subscription(
        &self,
        speaker_id: SpeakerId,
        service_type: ServiceType,
        callback_url: String,
        config: SubscriptionConfig,
    ) -> SubscriptionResult<Box<dyn ServiceSubscription>>;

    /// Check if a service type is supported by this factory
    fn supports_service(&self, service_type: ServiceType) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SpeakerId;

    // Mock implementation for testing
    struct MockSubscription {
        service_type: ServiceType,
        speaker_id: SpeakerId,
        subscription_id: Option<SubscriptionId>,
        active: bool,
        last_renewal: Option<SystemTime>,
        config: SubscriptionConfig,
        callback_url: String,
    }

    impl MockSubscription {
        fn new(service_type: ServiceType, speaker_id: SpeakerId, callback_url: String) -> Self {
            Self {
                service_type,
                speaker_id,
                subscription_id: None,
                active: false,
                last_renewal: None,
                config: SubscriptionConfig::default(),
                callback_url,
            }
        }
    }

    impl ServiceSubscription for MockSubscription {
        fn service_type(&self) -> ServiceType {
            self.service_type
        }

        fn speaker_id(&self) -> SpeakerId {
            self.speaker_id
        }

        fn subscribe(&mut self) -> SubscriptionResult<SubscriptionId> {
            let id = SubscriptionId::new();
            self.subscription_id = Some(id);
            self.active = true;
            self.last_renewal = Some(SystemTime::now());
            Ok(id)
        }

        fn unsubscribe(&mut self) -> SubscriptionResult<()> {
            self.subscription_id = None;
            self.active = false;
            self.last_renewal = None;
            Ok(())
        }

        fn renew(&mut self) -> SubscriptionResult<()> {
            if self.active {
                self.last_renewal = Some(SystemTime::now());
                Ok(())
            } else {
                Err(SubscriptionError::SubscriptionExpired)
            }
        }

        fn parse_event(&self, _event_xml: &str) -> SubscriptionResult<Vec<StateChange>> {
            // Mock implementation returns empty vec
            Ok(vec![])
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
    }

    #[test]
    fn test_subscription_lifecycle() {
        let speaker_id = SpeakerId::from_udn("uuid:RINCON_123456789::1");
        let mut subscription = MockSubscription::new(
            ServiceType::AVTransport,
            speaker_id,
            "http://localhost:8080/callback".to_string(),
        );

        // Initially not active
        assert!(!subscription.is_active());
        assert!(subscription.subscription_id().is_none());
        assert!(subscription.last_renewal().is_none());

        // Subscribe
        let sub_id = subscription.subscribe().unwrap();
        assert!(subscription.is_active());
        assert_eq!(subscription.subscription_id(), Some(sub_id));
        assert!(subscription.last_renewal().is_some());

        // Renew
        let old_renewal = subscription.last_renewal();
        std::thread::sleep(std::time::Duration::from_millis(1));
        subscription.renew().unwrap();
        assert!(subscription.last_renewal() > old_renewal);

        // Unsubscribe
        subscription.unsubscribe().unwrap();
        assert!(!subscription.is_active());
        assert!(subscription.subscription_id().is_none());
        assert!(subscription.last_renewal().is_none());
    }

    #[test]
    fn test_subscription_error_types() {
        // Test different error types
        let parse_err = SubscriptionError::EventParseError("test error".to_string());
        assert!(matches!(parse_err, SubscriptionError::EventParseError(_)));
        
        let expired_err = SubscriptionError::SubscriptionExpired;
        assert!(matches!(expired_err, SubscriptionError::SubscriptionExpired));
        
        let config_err = SubscriptionError::InvalidConfiguration("invalid".to_string());
        assert!(matches!(config_err, SubscriptionError::InvalidConfiguration(_)));
    }

    #[test]
    fn test_needs_renewal() {
        let speaker_id = SpeakerId::from_udn("uuid:RINCON_123456789::1");
        let mut subscription = MockSubscription::new(
            ServiceType::AVTransport,
            speaker_id,
            "http://localhost:8080/callback".to_string(),
        );

        // Not active, should not need renewal
        assert!(!subscription.needs_renewal());

        // Subscribe and check renewal logic
        subscription.subscribe().unwrap();
        
        // Should not need renewal immediately after subscribing
        assert!(!subscription.needs_renewal());
    }
}