use super::{parser::XmlParser, error::XmlParseResult, types::XmlRenderingControlData};
use quick_xml::events::Event;

/// RenderingControl service-specific extensions for the XML parser
impl<'a> XmlParser<'a> {
    /// Parse a complete RenderingControl event and extract all relevant data
    pub fn parse_rendering_control_event(&mut self) -> XmlParseResult<XmlRenderingControlData> {
        self.extract_rendering_control_properties()
    }

    /// Parse volume information from RenderingControl event XML
    /// 
    /// This method handles both direct property values and LastChange nested XML:
    /// - Direct: `<property><Volume>50</Volume></property>`
    /// - LastChange: `<property><LastChange>&lt;Event&gt;&lt;Volume val="50"/&gt;&lt;/Event&gt;</LastChange></property>`
    pub fn parse_volume(&mut self) -> XmlParseResult<Option<u8>> {
        let data = self.extract_rendering_control_properties()?;
        Ok(data.volume)
    }

    /// Parse mute state information from RenderingControl event XML
    /// 
    /// This method handles both direct property values and LastChange nested XML:
    /// - Direct: `<property><Mute>1</Mute></property>`
    /// - LastChange: `<property><LastChange>&lt;Event&gt;&lt;Mute val="0"/&gt;&lt;/Event&gt;</LastChange></property>`
    pub fn parse_mute_state(&mut self) -> XmlParseResult<Option<bool>> {
        let data = self.extract_rendering_control_properties()?;
        Ok(data.muted)
    }

    /// Internal method that does the actual parsing work in a single pass
    fn extract_rendering_control_properties(&mut self) -> XmlParseResult<XmlRenderingControlData> {
        let mut buffer = Vec::new();
        let mut volume = None;
        let mut muted = None;
        let mut in_property = false;
        let mut depth = 0;
        let mut lastchange_content = None;

        // First pass: look for direct properties and LastChange content
        loop {
            buffer.clear();
            match self.reader.read_event_into(&mut buffer)? {
                Event::Start(ref e) => {
                    if e.name().as_ref() == b"property" || e.name().as_ref().ends_with(b":property") {
                        in_property = true;
                        depth = 1;
                    } else if in_property {
                        if e.name().as_ref() == b"Volume" {
                            buffer.clear();
                            match self.reader.read_event_into(&mut buffer)? {
                                Event::Text(text) => {
                                    let content = text.unescape()?.into_owned();
                                    if let Ok(vol) = content.parse::<u8>() {
                                        volume = Some(vol);
                                    }
                                }
                                Event::CData(cdata) => {
                                    let content = String::from_utf8_lossy(&cdata);
                                    if let Ok(vol) = content.parse::<u8>() {
                                        volume = Some(vol);
                                    }
                                }
                                _ => {}
                            }
                        } else if e.name().as_ref() == b"Mute" {
                            buffer.clear();
                            match self.reader.read_event_into(&mut buffer)? {
                                Event::Text(text) => {
                                    let content = text.unescape()?.into_owned();
                                    muted = Some(Self::parse_boolean_value(&content));
                                }
                                Event::CData(cdata) => {
                                    let content = String::from_utf8_lossy(&cdata);
                                    muted = Some(Self::parse_boolean_value(&content));
                                }
                                _ => {}
                            }
                        } else if e.name().as_ref() == b"LastChange" {
                            buffer.clear();
                            match self.reader.read_event_into(&mut buffer)? {
                                Event::Text(text) => {
                                    lastchange_content = Some(text.unescape()?.into_owned());
                                }
                                Event::CData(cdata) => {
                                    lastchange_content = Some(String::from_utf8_lossy(&cdata).into_owned());
                                }
                                _ => {}
                            }
                        } else {
                            depth += 1;
                        }
                    }
                }
                Event::End(ref e) => {
                    if in_property {
                        depth -= 1;
                        if depth == 0 && (e.name().as_ref() == b"property" || e.name().as_ref().ends_with(b":property")) {
                            in_property = false;
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        // If we have LastChange content and missing properties, parse the nested XML
        if let Some(lastchange_xml) = lastchange_content {
            let decoded_xml = Self::decode_entities(&lastchange_xml);
            
            if volume.is_none() {
                let mut nested_parser = XmlParser::new(&decoded_xml);
                if let Some(volume_str) = nested_parser.extract_nested_property_value("Volume")? {
                    if let Ok(vol) = volume_str.parse::<u8>() {
                        volume = Some(vol);
                    }
                }
            }

            if muted.is_none() {
                let mut nested_parser = XmlParser::new(&decoded_xml);
                if let Some(mute_str) = nested_parser.extract_nested_property_value("Mute")? {
                    muted = Some(Self::parse_boolean_value(&mute_str));
                }
            }
        }

        Ok(XmlRenderingControlData { volume, muted })
    }

    /// Helper method to parse boolean values from UPnP XML
    /// UPnP uses "1"/"0" or "true"/"false" for boolean values
    fn parse_boolean_value(value: &str) -> bool {
        match value.to_lowercase().as_str() {
            "1" | "true" => true,
            "0" | "false" => false,
            _ => false, // Default to false for unknown values
        }
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