use super::types::XmlRenderingControlData;
use crate::xml::error::XmlParseResult;

/// Parse a complete RenderingControl event using serde
pub fn parse_rendering_control_event(xml: &str) -> XmlParseResult<XmlRenderingControlData> {
    // Use the original XML parser to get the data
    let original_data = crate::xml::parser::XmlParser::parse_rendering_control_serde(xml)?;

    // The data is already in the correct format, just return it
    Ok(XmlRenderingControlData {
        volume: original_data.volume,
        muted: original_data.muted,
    })
}

/// Parse volume information using serde
pub fn parse_volume(xml: &str) -> XmlParseResult<Option<u8>> {
    let data = parse_rendering_control_event(xml)?;
    Ok(data.volume)
}

/// Parse mute state information using serde
pub fn parse_mute_state(xml: &str) -> XmlParseResult<Option<bool>> {
    let data = parse_rendering_control_event(xml)?;
    Ok(data.muted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_volume_direct_property() {
        let xml = r#"
            <property>
                <Volume>75</Volume>
            </property>
        "#;
        let result = parse_volume(xml).unwrap();
        assert_eq!(result, Some(75));
    }

    #[test]
    fn test_parse_volume_lastchange() {
        let xml = r#"
            <property>
                <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"&gt;&lt;InstanceID val="0"&gt;&lt;Volume val="50"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
            </property>
        "#;
        let result = parse_volume(xml).unwrap();
        assert_eq!(result, Some(50));
    }

    #[test]
    fn test_parse_volume_missing() {
        let xml = r#"
            <property>
                <Mute>1</Mute>
            </property>
        "#;
        let result = parse_volume(xml).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_mute_state_direct_property_true() {
        let xml = r#"
            <property>
                <Mute>1</Mute>
            </property>
        "#;
        let result = parse_mute_state(xml).unwrap();
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_parse_mute_state_direct_property_false() {
        let xml = r#"
            <property>
                <Mute>0</Mute>
            </property>
        "#;
        let result = parse_mute_state(xml).unwrap();
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_parse_mute_state_lastchange() {
        let xml = r#"
            <property>
                <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"&gt;&lt;InstanceID val="0"&gt;&lt;Mute val="1"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
            </property>
        "#;
        let result = parse_mute_state(xml).unwrap();
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_parse_mute_state_missing() {
        let xml = r#"
            <property>
                <Volume>50</Volume>
            </property>
        "#;
        let result = parse_mute_state(xml).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_rendering_control_event() {
        let xml = r#"
            <property>
                <Volume>75</Volume>
                <Mute>1</Mute>
            </property>
        "#;
        let result = parse_rendering_control_event(xml).unwrap();
        assert_eq!(result.volume, Some(75));
        assert_eq!(result.muted, Some(true));
    }

    #[test]
    fn test_parse_rendering_control_event_partial() {
        let xml = r#"
            <property>
                <Volume>25</Volume>
            </property>
        "#;
        let result = parse_rendering_control_event(xml).unwrap();
        assert_eq!(result.volume, Some(25));
        assert_eq!(result.muted, None);
    }
}
