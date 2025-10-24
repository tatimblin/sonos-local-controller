// Module declarations
pub mod av_transport;
pub mod error;
pub mod parser;
pub mod rendering_control;
pub mod types;
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
        // Test serde-based parsing integration
        let xml = r#"
            <property>
                <Volume>50</Volume>
                <Mute>0</Mute>
            </property>
        "#;
        
        let result = XmlParser::parse_rendering_control_serde(xml).unwrap();
        assert_eq!(result.volume, Some(50));
        assert_eq!(result.muted, Some(false));
    }
}