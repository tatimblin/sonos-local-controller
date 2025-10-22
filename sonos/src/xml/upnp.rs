use super::{parser::XmlParser, error::XmlParseResult};
use quick_xml::events::Event;

/// UPnP-specific extensions for the XML parser
impl<'a> XmlParser<'a> {
    /// Extract a property value from UPnP event XML, supporting both namespaced and non-namespaced properties
    /// 
    /// This method handles patterns like:
    /// - `<property><Volume>50</Volume></property>`
    /// - `<e:property><Volume>50</Volume></e:property>`
    /// - `<property xmlns="..."><Volume>50</Volume></property>`
    pub fn extract_property_value(&mut self, property_name: &str) -> XmlParseResult<Option<String>> {
        let mut buffer = Vec::new();
        let mut in_property = false;
        let mut depth = 0;

        loop {
            buffer.clear();
            match self.reader.read_event_into(&mut buffer)? {
                Event::Start(ref e) => {
                    // Check if this is a property element (with or without namespace)
                    if e.name().as_ref() == b"property" || e.name().as_ref().ends_with(b":property") {
                        in_property = true;
                        depth = 1;
                    } else if in_property && e.name().as_ref() == property_name.as_bytes() {
                        // Found the target property, read its content
                        buffer.clear();
                        match self.reader.read_event_into(&mut buffer)? {
                            Event::Text(text) => {
                                let content = text.unescape()?.into_owned();
                                return Ok(Some(Self::decode_entities(&content)));
                            }
                            Event::CData(cdata) => {
                                let content = String::from_utf8_lossy(&cdata).into_owned();
                                return Ok(Some(content));
                            }
                            Event::End(_) => return Ok(Some(String::new())),
                            _ => {}
                        }
                    } else if in_property {
                        depth += 1;
                    }
                }
                Event::End(ref e) => {
                    if in_property {
                        depth -= 1;
                        if depth == 0 {
                            // Check if this is the end of a property element
                            if e.name().as_ref() == b"property" || e.name().as_ref().ends_with(b":property") {
                                in_property = false;
                            }
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        Ok(None)
    }

    /// Extract a property value from nested XML within a LastChange property
    /// 
    /// This method handles the complex nested XML structure found in UPnP LastChange events:
    /// ```xml
    /// <property>
    ///   <LastChange>
    ///     &lt;Event xmlns="..."&gt;
    ///       &lt;InstanceID val="0"&gt;
    ///         &lt;Volume val="50"/&gt;
    ///       &lt;/InstanceID&gt;
    ///     &lt;/Event&gt;
    ///   </LastChange>
    /// </property>
    /// ```
    pub fn extract_lastchange_property(&mut self, property_name: &str) -> XmlParseResult<Option<String>> {
        // First, find the LastChange element
        if let Some(lastchange_content) = self.extract_property_value("LastChange")? {
            // Decode the nested XML entities
            let decoded_xml = Self::decode_entities(&lastchange_content);
            
            // Parse the nested XML to find the property
            let mut nested_parser = XmlParser::new(&decoded_xml);
            return nested_parser.extract_nested_property_value(property_name);
        }

        Ok(None)
    }

    /// Helper method to extract property values from nested XML structures
    /// Used internally by extract_lastchange_property
    pub(crate) fn extract_nested_property_value(&mut self, property_name: &str) -> XmlParseResult<Option<String>> {
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            match self.reader.read_event_into(&mut buffer)? {
                Event::Start(ref e) | Event::Empty(ref e) => {
                    // Compare the element name directly with bytes to avoid temporary string creation
                    if e.name().as_ref() == property_name.as_bytes() {
                        // Check if this is a self-closing element with a 'val' attribute
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"val" {
                                let value = attr.unescape_value()?.into_owned();
                                return Ok(Some(Self::decode_entities(&value)));
                            }
                        }
                        
                        // If not self-closing, read the content
                        if !matches!(self.reader.read_event_into(&mut buffer)?, Event::End(_)) {
                            buffer.clear();
                            match self.reader.read_event_into(&mut buffer)? {
                                Event::Text(text) => {
                                    let content = text.unescape()?.into_owned();
                                    return Ok(Some(Self::decode_entities(&content)));
                                }
                                Event::CData(cdata) => {
                                    let content = String::from_utf8_lossy(&cdata).into_owned();
                                    return Ok(Some(content));
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_property_value_simple() {
        let xml = r#"
            <property>
                <Volume>50</Volume>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.extract_property_value("Volume").unwrap();
        assert_eq!(result, Some("50".to_string()));
    }

    #[test]
    fn test_extract_property_value_namespaced() {
        let xml = r#"
            <e:property xmlns:e="urn:schemas-upnp-org:event-1-0">
                <Volume>75</Volume>
            </e:property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.extract_property_value("Volume").unwrap();
        assert_eq!(result, Some("75".to_string()));
    }

    #[test]
    fn test_extract_property_value_with_entities() {
        let xml = r#"
            <property>
                <Title>&lt;Test &amp; Song&gt;</Title>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.extract_property_value("Title").unwrap();
        assert_eq!(result, Some("<Test & Song>".to_string()));
    }

    #[test]
    fn test_extract_property_value_missing() {
        let xml = r#"
            <property>
                <Volume>50</Volume>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.extract_property_value("Mute").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_lastchange_property() {
        let xml = r#"
            <property>
                <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"&gt;&lt;InstanceID val="0"&gt;&lt;Volume val="50"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.extract_lastchange_property("Volume").unwrap();
        assert_eq!(result, Some("50".to_string()));
    }

    #[test]
    fn test_extract_lastchange_property_missing() {
        let xml = r#"
            <property>
                <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"&gt;&lt;InstanceID val="0"&gt;&lt;Volume val="50"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.extract_lastchange_property("Mute").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_nested_property_value() {
        let nested_xml = r#"<Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"><InstanceID val="0"><Volume val="75"/></InstanceID></Event>"#;
        let mut parser = XmlParser::new(nested_xml);
        let result = parser.extract_nested_property_value("Volume").unwrap();
        assert_eq!(result, Some("75".to_string()));
    }
}