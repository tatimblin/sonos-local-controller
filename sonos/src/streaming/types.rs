use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Represents different UPnP service types that can be subscribed to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceType {
    AVTransport,
    RenderingControl,
    ContentDirectory,
}

impl ServiceType {
    /// Get the UPnP service type string for SOAP requests
    pub fn service_type_urn(&self) -> &'static str {
        match self {
            ServiceType::AVTransport => "urn:schemas-upnp-org:service:AVTransport:1",
            ServiceType::RenderingControl => "urn:schemas-upnp-org:service:RenderingControl:1",
            ServiceType::ContentDirectory => "urn:schemas-upnp-org:service:ContentDirectory:1",
        }
    }

    /// Get the control URL path for this service
    pub fn control_url(&self) -> &'static str {
        match self {
            ServiceType::AVTransport => "/MediaRenderer/AVTransport/Control",
            ServiceType::RenderingControl => "/MediaRenderer/RenderingControl/Control",
            ServiceType::ContentDirectory => "/MediaServer/ContentDirectory/Control",
        }
    }

    /// Get the event subscription URL path for this service
    pub fn event_sub_url(&self) -> &'static str {
        match self {
            ServiceType::AVTransport => "/MediaRenderer/AVTransport/Event",
            ServiceType::RenderingControl => "/MediaRenderer/RenderingControl/Event",
            ServiceType::ContentDirectory => "/MediaServer/ContentDirectory/Event",
        }
    }
}

/// Unique identifier for a UPnP subscription
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(Uuid);

impl SubscriptionId {
    /// Create a new random subscription ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a subscription ID from a UUID string
    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    /// Get the UUID as a string
    pub fn as_string(&self) -> String {
        self.0.to_string()
    }

    /// Get the inner UUID
    pub fn uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for SubscriptionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Configuration for the overall streaming system
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Size of the event buffer for the unified event stream
    pub buffer_size: usize,
    /// Default timeout for UPnP subscriptions
    pub subscription_timeout: Duration,
    /// Maximum number of retry attempts for failed subscriptions
    pub retry_attempts: u32,
    /// Base duration for exponential backoff retry strategy
    pub retry_backoff: Duration,
    /// List of service types to enable for streaming
    pub enabled_services: Vec<ServiceType>,
    /// Port range for the HTTP callback server (start, end)
    pub callback_port_range: (u16, u16),
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1000,
            subscription_timeout: Duration::from_secs(1800), // 30 minutes
            retry_attempts: 3,
            retry_backoff: Duration::from_secs(1),
            enabled_services: vec![ServiceType::AVTransport],
            callback_port_range: (8080, 8090),
        }
    }
}

impl StreamConfig {
    /// Create a new StreamConfig with validation
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a minimal configuration for testing or low-resource environments
    pub fn minimal() -> Self {
        Self {
            buffer_size: 100,
            subscription_timeout: Duration::from_secs(300), // 5 minutes
            retry_attempts: 1,
            retry_backoff: Duration::from_millis(500),
            enabled_services: vec![ServiceType::AVTransport],
            callback_port_range: (8080, 8085),
        }
    }

    /// Create a comprehensive configuration with all services enabled
    pub fn comprehensive() -> Self {
        Self {
            buffer_size: 5000,
            subscription_timeout: Duration::from_secs(3600), // 1 hour
            retry_attempts: 5,
            retry_backoff: Duration::from_secs(2),
            enabled_services: vec![
                ServiceType::AVTransport,
                ServiceType::RenderingControl,
                ServiceType::ContentDirectory,
            ],
            callback_port_range: (8080, 8100),
        }
    }

    /// Create a configuration optimized for production use
    pub fn production() -> Self {
        Self {
            buffer_size: 2000,
            subscription_timeout: Duration::from_secs(1800), // 30 minutes
            retry_attempts: 3,
            retry_backoff: Duration::from_secs(1),
            enabled_services: vec![ServiceType::AVTransport, ServiceType::RenderingControl],
            callback_port_range: (8080, 8090),
        }
    }

    /// Set the buffer size with validation
    pub fn with_buffer_size(mut self, size: usize) -> Result<Self, String> {
        if size == 0 {
            return Err("Buffer size must be greater than 0".to_string());
        }
        if size > 100_000 {
            return Err("Buffer size too large (max 100,000)".to_string());
        }
        self.buffer_size = size;
        Ok(self)
    }

    /// Set the subscription timeout with validation
    pub fn with_subscription_timeout(mut self, timeout: Duration) -> Result<Self, String> {
        if timeout.as_secs() < 60 {
            return Err("Subscription timeout must be at least 60 seconds".to_string());
        }
        if timeout.as_secs() > 86400 {
            return Err("Subscription timeout too long (max 24 hours)".to_string());
        }
        self.subscription_timeout = timeout;
        Ok(self)
    }

    /// Set the retry attempts with validation
    pub fn with_retry_attempts(mut self, attempts: u32) -> Result<Self, String> {
        if attempts > 10 {
            return Err("Too many retry attempts (max 10)".to_string());
        }
        self.retry_attempts = attempts;
        Ok(self)
    }

    /// Set the retry backoff duration
    pub fn with_retry_backoff(mut self, backoff: Duration) -> Self {
        self.retry_backoff = backoff;
        self
    }

    /// Set the enabled services
    pub fn with_enabled_services(mut self, services: Vec<ServiceType>) -> Self {
        self.enabled_services = services;
        self
    }

    /// Set the callback port range with validation
    pub fn with_callback_port_range(mut self, start: u16, end: u16) -> Result<Self, String> {
        if start >= end {
            return Err("Port range start must be less than end".to_string());
        }
        if start < 1024 {
            return Err("Port range start must be >= 1024".to_string());
        }
        // Note: u16 max is 65535, so this check is always false but kept for clarity
        #[allow(unused_comparisons)]
        if end > 65535 {
            return Err("Port range end must be <= 65535".to_string());
        }
        self.callback_port_range = (start, end);
        Ok(self)
    }

    /// Validate the entire configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.buffer_size == 0 {
            return Err("Buffer size must be greater than 0".to_string());
        }
        if self.subscription_timeout.as_secs() < 60 {
            return Err("Subscription timeout must be at least 60 seconds".to_string());
        }
        if self.retry_attempts > 10 {
            return Err("Too many retry attempts (max 10)".to_string());
        }
        if self.callback_port_range.0 >= self.callback_port_range.1 {
            return Err("Invalid port range".to_string());
        }
        if self.enabled_services.is_empty() {
            return Err("At least one service must be enabled".to_string());
        }
        Ok(())
    }
}

/// Configuration for individual service subscriptions
#[derive(Debug, Clone)]
pub struct SubscriptionConfig {
    /// Subscription timeout in seconds
    pub timeout_seconds: u32,
    /// Threshold before expiry to renew subscription
    pub renewal_threshold: Duration,
    /// Maximum number of retry attempts for this subscription
    pub max_retry_attempts: u32,
    /// Base duration for exponential backoff
    pub retry_backoff_base: Duration,
}

impl Default for SubscriptionConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 1800, // 30 minutes
            renewal_threshold: Duration::from_secs(300), // 5 minutes before expiry
            max_retry_attempts: 3,
            retry_backoff_base: Duration::from_secs(1),
        }
    }
}

impl SubscriptionConfig {
    /// Create a new SubscriptionConfig from StreamConfig
    pub fn from_stream_config(stream_config: &StreamConfig) -> Self {
        Self {
            timeout_seconds: stream_config.subscription_timeout.as_secs() as u32,
            renewal_threshold: Duration::from_secs(300),
            max_retry_attempts: stream_config.retry_attempts,
            retry_backoff_base: stream_config.retry_backoff,
        }
    }
}

/// Raw event data received from UPnP notifications
#[derive(Debug, Clone)]
pub struct RawEvent {
    /// The subscription ID this event belongs to
    pub subscription_id: SubscriptionId,
    /// The raw XML content of the event
    pub event_xml: String,
    /// Timestamp when the event was received
    pub timestamp: SystemTime,
}

impl RawEvent {
    /// Create a new RawEvent
    pub fn new(subscription_id: SubscriptionId, event_xml: String) -> Self {
        Self {
            subscription_id,
            event_xml,
            timestamp: SystemTime::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_type_urns() {
        assert_eq!(
            ServiceType::AVTransport.service_type_urn(),
            "urn:schemas-upnp-org:service:AVTransport:1"
        );
        assert_eq!(
            ServiceType::RenderingControl.service_type_urn(),
            "urn:schemas-upnp-org:service:RenderingControl:1"
        );
        assert_eq!(
            ServiceType::ContentDirectory.service_type_urn(),
            "urn:schemas-upnp-org:service:ContentDirectory:1"
        );
    }

    #[test]
    fn test_service_type_urls() {
        assert_eq!(
            ServiceType::AVTransport.control_url(),
            "/MediaRenderer/AVTransport/Control"
        );
        assert_eq!(
            ServiceType::AVTransport.event_sub_url(),
            "/MediaRenderer/AVTransport/Event"
        );
    }

    #[test]
    fn test_subscription_id() {
        let id1 = SubscriptionId::new();
        let id2 = SubscriptionId::new();
        
        // Different IDs should not be equal
        assert_ne!(id1, id2);
        
        // String conversion should work
        let id_str = id1.as_string();
        let id3 = SubscriptionId::from_string(&id_str).unwrap();
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_stream_config_default() {
        let config = StreamConfig::default();
        assert_eq!(config.buffer_size, 1000);
        assert_eq!(config.subscription_timeout, Duration::from_secs(1800));
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.enabled_services, vec![ServiceType::AVTransport]);
        assert_eq!(config.callback_port_range, (8080, 8090));
    }

    #[test]
    fn test_stream_config_validation() {
        let config = StreamConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid buffer size
        let invalid_config = StreamConfig::default().with_buffer_size(0);
        assert!(invalid_config.is_err());

        // Test invalid timeout
        let invalid_config = StreamConfig::default().with_subscription_timeout(Duration::from_secs(30));
        assert!(invalid_config.is_err());

        // Test invalid port range
        let invalid_config = StreamConfig::default().with_callback_port_range(8080, 8080);
        assert!(invalid_config.is_err());
    }

    #[test]
    fn test_subscription_config_from_stream_config() {
        let stream_config = StreamConfig::default();
        let sub_config = SubscriptionConfig::from_stream_config(&stream_config);
        
        assert_eq!(sub_config.timeout_seconds, 1800);
        assert_eq!(sub_config.max_retry_attempts, 3);
    }

    #[test]
    fn test_raw_event() {
        let sub_id = SubscriptionId::new();
        let xml = "<event>test</event>".to_string();
        let event = RawEvent::new(sub_id, xml.clone());
        
        assert_eq!(event.subscription_id, sub_id);
        assert_eq!(event.event_xml, xml);
        assert!(event.timestamp <= SystemTime::now());
    }
}