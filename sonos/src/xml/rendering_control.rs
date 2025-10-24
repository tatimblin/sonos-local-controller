use super::{parser::XmlParser, error::XmlParseResult, types::XmlRenderingControlData};

/// RenderingControl service-specific extensions for the XML parser
impl<'a> XmlParser<'a> {
    /// Parse a complete RenderingControl event and extract all relevant data using serde
    pub fn parse_rendering_control_event(&mut self) -> XmlParseResult<XmlRenderingControlData> {
        let xml_content = std::str::from_utf8(self.reader.get_ref())
            .map_err(|e| super::error::XmlParseError::SyntaxError(format!("Invalid UTF-8: {}", e)))?;
        
        XmlParser::parse_rendering_control_serde(xml_content)
    }

    /// Parse volume information from RenderingControl event XML using serde
    pub fn parse_volume(&mut self) -> XmlParseResult<Option<u8>> {
        let data = self.parse_rendering_control_event()?;
        Ok(data.volume)
    }

    /// Parse mute state information from RenderingControl event XML using serde
    pub fn parse_mute_state(&mut self) -> XmlParseResult<Option<bool>> {
        let data = self.parse_rendering_control_event()?;
        Ok(data.muted)
    }
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
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_volume().unwrap();
        assert_eq!(result, Some(75));
    }

    #[test]
    fn test_parse_volume_lastchange() {
        let xml = r#"
            <property>
                <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"&gt;&lt;InstanceID val="0"&gt;&lt;Volume val="50"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_volume().unwrap();
        assert_eq!(result, Some(50));
    }

    #[test]
    fn test_parse_volume_missing() {
        let xml = r#"
            <property>
                <Mute>1</Mute>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_volume().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_mute_state_direct_property_true() {
        let xml = r#"
            <property>
                <Mute>1</Mute>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_mute_state().unwrap();
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_parse_mute_state_direct_property_false() {
        let xml = r#"
            <property>
                <Mute>0</Mute>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_mute_state().unwrap();
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_parse_mute_state_lastchange() {
        let xml = r#"
            <property>
                <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"&gt;&lt;InstanceID val="0"&gt;&lt;Mute val="1"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_mute_state().unwrap();
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_parse_mute_state_missing() {
        let xml = r#"
            <property>
                <Volume>50</Volume>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_mute_state().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_boolean_value() {
        assert_eq!(XmlParser::parse_boolean_value("1"), true);
        assert_eq!(XmlParser::parse_boolean_value("0"), false);
        assert_eq!(XmlParser::parse_boolean_value("true"), true);
        assert_eq!(XmlParser::parse_boolean_value("false"), false);
        assert_eq!(XmlParser::parse_boolean_value("TRUE"), true);
        assert_eq!(XmlParser::parse_boolean_value("FALSE"), false);
        assert_eq!(XmlParser::parse_boolean_value("invalid"), false);
    }

    #[test]
    fn test_parse_rendering_control_event() {
        let xml = r#"
            <property>
                <Volume>75</Volume>
                <Mute>1</Mute>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_rendering_control_event().unwrap();
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
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_rendering_control_event().unwrap();
        assert_eq!(result.volume, Some(25));
        assert_eq!(result.muted, None);
    }
}