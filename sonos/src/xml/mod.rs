// Module declarations
pub mod av_transport;
pub mod error;
pub mod parser;
pub mod rendering_control;
pub mod types;
pub mod upnp;
pub mod zone_group_topology;

// Re-export public API for easy access
pub use error::{XmlParseError, XmlParseResult};
pub use parser::XmlParser;
pub use types::{
    XmlAVTransportData, XmlDidlMetadata, XmlRenderingControlData, XmlZoneGroupData,
    XmlZoneGroupMember,
};


#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::streaming::subscription::SubscriptionError;

    #[test]
    fn test_error_conversion_integration() {
        // Test XmlParseError to SubscriptionError conversion across modules
        let xml_error = XmlParseError::MissingElement {
            element: "Volume".to_string(),
        };
        let subscription_error: SubscriptionError = xml_error.into();

        match subscription_error {
            SubscriptionError::XmlParseError(msg) => {
                assert!(msg.contains("Missing required element: Volume"));
            }
            _ => panic!("Expected XmlParseError variant"),
        }
    }

    #[test]
    fn test_module_integration() {
        // Test that all modules work together
        let xml = "<root><volume>50</volume></root>";
        let mut parser = XmlParser::new(xml);
        
        let volume = parser.find_element("volume").unwrap();
        assert_eq!(volume, Some("50".to_string()));

        // Test data structure creation
        let rendering_data = XmlRenderingControlData {
            volume: Some(50),
            muted: Some(false),
        };
        assert_eq!(rendering_data.volume, Some(50));
    }
}