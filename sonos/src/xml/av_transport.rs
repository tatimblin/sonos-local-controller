use super::{parser::XmlParser, error::XmlParseResult, types::{XmlAVTransportData, XmlDidlMetadata}};
use quick_xml::events::Event;

/// AVTransport service-specific extensions for the XML parser
impl<'a> XmlParser<'a> {
    /// Parse a complete AVTransport event and extract all relevant data
    pub fn parse_av_transport_event(&mut self) -> XmlParseResult<XmlAVTransportData> {
        self.extract_av_transport_properties()
    }

    /// Parse transport state information from AVTransport event XML
    /// 
    /// This method handles both direct property values and LastChange nested XML:
    /// - Direct: `<property><TransportState>PLAYING</TransportState></property>`
    /// - LastChange: `<property><LastChange>&lt;Event&gt;&lt;TransportState val="PLAYING"/&gt;&lt;/Event&gt;</LastChange></property>`
    pub fn parse_transport_state(&mut self) -> XmlParseResult<Option<String>> {
        let data = self.extract_av_transport_properties()?;
        Ok(data.transport_state)
    }

    /// Extract DIDL metadata from DIDL-Lite XML content
    /// 
    /// This method parses DIDL-Lite XML to extract track metadata:
    /// ```xml
    /// <DIDL-Lite>
    ///   <item>
    ///     <dc:title>Song Title</dc:title>
    ///     <dc:creator>Artist Name</dc:creator>
    ///     <upnp:album>Album Name</upnp:album>
    ///   </item>
    /// </DIDL-Lite>
    /// ```
    pub fn extract_didl_metadata(&mut self, metadata_xml: &str) -> XmlParseResult<XmlDidlMetadata> {
        let mut parser = XmlParser::new(metadata_xml);
        parser.parse_didl_metadata()
    }

    /// Parse duration string from various formats
    /// 
    /// Supports formats like:
    /// - "0:03:45" (H:MM:SS)
    /// - "00:03:45" (HH:MM:SS)
    /// - "3:45" (MM:SS)
    /// - "225" (seconds)
    pub fn parse_duration(&self, duration_str: &str) -> Option<u64> {
        if duration_str.is_empty() {
            return None;
        }

        // Try parsing as seconds first
        if let Ok(seconds) = duration_str.parse::<u64>() {
            return Some(seconds);
        }

        // Try parsing as time format (H:MM:SS or MM:SS)
        let parts: Vec<&str> = duration_str.split(':').collect();
        match parts.len() {
            2 => {
                // MM:SS format
                if let (Ok(minutes), Ok(seconds)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                    Some(minutes * 60 + seconds)
                } else {
                    None
                }
            }
            3 => {
                // H:MM:SS format
                if let (Ok(hours), Ok(minutes), Ok(seconds)) = (
                    parts[0].parse::<u64>(),
                    parts[1].parse::<u64>(),
                    parts[2].parse::<u64>(),
                ) {
                    Some(hours * 3600 + minutes * 60 + seconds)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Internal method that does the actual AVTransport parsing work in a single pass
    fn extract_av_transport_properties(&mut self) -> XmlParseResult<XmlAVTransportData> {
        let mut buffer = Vec::new();
        let mut transport_state = None;
        let mut current_track_metadata = None;
        let mut current_track_duration = None;
        let mut current_track_uri = None;
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
                        if e.name().as_ref() == b"TransportState" {
                            buffer.clear();
                            match self.reader.read_event_into(&mut buffer)? {
                                Event::Text(text) => {
                                    transport_state = Some(text.unescape()?.into_owned());
                                }
                                Event::CData(cdata) => {
                                    transport_state = Some(String::from_utf8_lossy(&cdata).into_owned());
                                }
                                _ => {}
                            }
                        } else if e.name().as_ref() == b"CurrentTrackMetaData" {
                            buffer.clear();
                            match self.reader.read_event_into(&mut buffer)? {
                                Event::Text(text) => {
                                    let metadata_xml = text.unescape()?.into_owned();
                                    let decoded_xml = Self::decode_entities(&metadata_xml);
                                    let mut metadata_parser = XmlParser::new(&decoded_xml);
                                    current_track_metadata = Some(metadata_parser.parse_didl_metadata()?);
                                }
                                Event::CData(cdata) => {
                                    let metadata_xml = String::from_utf8_lossy(&cdata);
                                    let mut metadata_parser = XmlParser::new(&metadata_xml);
                                    current_track_metadata = Some(metadata_parser.parse_didl_metadata()?);
                                }
                                _ => {}
                            }
                        } else if e.name().as_ref() == b"CurrentTrackDuration" {
                            buffer.clear();
                            match self.reader.read_event_into(&mut buffer)? {
                                Event::Text(text) => {
                                    current_track_duration = Some(text.unescape()?.into_owned());
                                }
                                Event::CData(cdata) => {
                                    current_track_duration = Some(String::from_utf8_lossy(&cdata).into_owned());
                                }
                                _ => {}
                            }
                        } else if e.name().as_ref() == b"CurrentTrackURI" {
                            buffer.clear();
                            match self.reader.read_event_into(&mut buffer)? {
                                Event::Text(text) => {
                                    current_track_uri = Some(text.unescape()?.into_owned());
                                }
                                Event::CData(cdata) => {
                                    current_track_uri = Some(String::from_utf8_lossy(&cdata).into_owned());
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
            
            if transport_state.is_none() {
                let mut nested_parser = XmlParser::new(&decoded_xml);
                transport_state = nested_parser.extract_nested_property_value("TransportState")?;
            }

            if current_track_metadata.is_none() {
                let mut nested_parser = XmlParser::new(&decoded_xml);
                if let Some(metadata_xml) = nested_parser.extract_nested_property_value("CurrentTrackMetaData")? {
                    let decoded_metadata = Self::decode_entities(&metadata_xml);
                    let mut metadata_parser = XmlParser::new(&decoded_metadata);
                    current_track_metadata = Some(metadata_parser.parse_didl_metadata()?);
                }
            }

            if current_track_duration.is_none() {
                let mut nested_parser = XmlParser::new(&decoded_xml);
                current_track_duration = nested_parser.extract_nested_property_value("CurrentTrackDuration")?;
            }

            if current_track_uri.is_none() {
                let mut nested_parser = XmlParser::new(&decoded_xml);
                current_track_uri = nested_parser.extract_nested_property_value("CurrentTrackURI")?;
            }
        }

        Ok(XmlAVTransportData {
            transport_state,
            current_track_metadata,
            current_track_duration,
            current_track_uri,
        })
    }

    /// Parse DIDL-Lite metadata XML
    fn parse_didl_metadata(&mut self) -> XmlParseResult<XmlDidlMetadata> {
        let mut buffer = Vec::new();
        let mut title = None;
        let mut artist = None;
        let mut album = None;

        loop {
            buffer.clear();
            match self.reader.read_event_into(&mut buffer)? {
                Event::Start(ref e) => {
                    if e.name().as_ref() == b"title" || e.name().as_ref().ends_with(b":title") {
                        buffer.clear();
                        match self.reader.read_event_into(&mut buffer)? {
                            Event::Text(text) => {
                                title = Some(Self::decode_entities(&text.unescape()?.into_owned()));
                            }
                            Event::CData(cdata) => {
                                title = Some(String::from_utf8_lossy(&cdata).into_owned());
                            }
                            _ => {}
                        }
                    } else if e.name().as_ref() == b"creator" || e.name().as_ref().ends_with(b":creator") {
                        buffer.clear();
                        match self.reader.read_event_into(&mut buffer)? {
                            Event::Text(text) => {
                                artist = Some(Self::decode_entities(&text.unescape()?.into_owned()));
                            }
                            Event::CData(cdata) => {
                                artist = Some(String::from_utf8_lossy(&cdata).into_owned());
                            }
                            _ => {}
                        }
                    } else if e.name().as_ref() == b"album" || e.name().as_ref().ends_with(b":album") {
                        buffer.clear();
                        match self.reader.read_event_into(&mut buffer)? {
                            Event::Text(text) => {
                                album = Some(Self::decode_entities(&text.unescape()?.into_owned()));
                            }
                            Event::CData(cdata) => {
                                album = Some(String::from_utf8_lossy(&cdata).into_owned());
                            }
                            _ => {}
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        Ok(XmlDidlMetadata { title, artist, album })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_transport_state_direct_property() {
        let xml = r#"
            <property>
                <TransportState>PLAYING</TransportState>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_transport_state().unwrap();
        assert_eq!(result, Some("PLAYING".to_string()));
    }

    #[test]
    fn test_parse_transport_state_lastchange() {
        let xml = r#"
            <property>
                <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/AVT/"&gt;&lt;InstanceID val="0"&gt;&lt;TransportState val="PAUSED_PLAYBACK"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_transport_state().unwrap();
        assert_eq!(result, Some("PAUSED_PLAYBACK".to_string()));
    }

    #[test]
    fn test_parse_duration_seconds() {
        let parser = XmlParser::new("");
        assert_eq!(parser.parse_duration("225"), Some(225));
    }

    #[test]
    fn test_parse_duration_mm_ss() {
        let parser = XmlParser::new("");
        assert_eq!(parser.parse_duration("3:45"), Some(225));
    }

    #[test]
    fn test_parse_duration_hh_mm_ss() {
        let parser = XmlParser::new("");
        assert_eq!(parser.parse_duration("1:03:45"), Some(3825));
        assert_eq!(parser.parse_duration("00:03:45"), Some(225));
    }

    #[test]
    fn test_parse_duration_invalid() {
        let parser = XmlParser::new("");
        assert_eq!(parser.parse_duration(""), None);
        assert_eq!(parser.parse_duration("invalid"), None);
        assert_eq!(parser.parse_duration("1:2:3:4"), None);
    }

    #[test]
    fn test_extract_didl_metadata() {
        let didl_xml = r#"
            <DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/">
                <item>
                    <dc:title>Test Song</dc:title>
                    <dc:creator>Test Artist</dc:creator>
                    <upnp:album>Test Album</upnp:album>
                </item>
            </DIDL-Lite>
        "#;
        let mut parser = XmlParser::new("");
        let result = parser.extract_didl_metadata(didl_xml).unwrap();
        assert_eq!(result.title, Some("Test Song".to_string()));
        assert_eq!(result.artist, Some("Test Artist".to_string()));
        assert_eq!(result.album, Some("Test Album".to_string()));
    }

    #[test]
    fn test_extract_didl_metadata_partial() {
        let didl_xml = r#"
            <DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/">
                <item>
                    <dc:title>Test Song</dc:title>
                </item>
            </DIDL-Lite>
        "#;
        let mut parser = XmlParser::new("");
        let result = parser.extract_didl_metadata(didl_xml).unwrap();
        assert_eq!(result.title, Some("Test Song".to_string()));
        assert_eq!(result.artist, None);
        assert_eq!(result.album, None);
    }

    #[test]
    fn test_parse_av_transport_event() {
        let xml = r#"
            <property>
                <TransportState>PLAYING</TransportState>
                <CurrentTrackDuration>0:03:45</CurrentTrackDuration>
                <CurrentTrackURI>x-sonos-spotify:spotify%3atrack%3a123</CurrentTrackURI>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_av_transport_event().unwrap();
        assert_eq!(result.transport_state, Some("PLAYING".to_string()));
        assert_eq!(result.current_track_duration, Some("0:03:45".to_string()));
        assert_eq!(result.current_track_uri, Some("x-sonos-spotify:spotify%3atrack%3a123".to_string()));
        assert_eq!(result.current_track_metadata, None);
    }

    #[test]
    fn test_parse_av_transport_event_with_metadata() {
        let xml = r#"
            <property>
                <TransportState>PLAYING</TransportState>
                <CurrentTrackMetaData>&lt;DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/"&gt;&lt;item&gt;&lt;dc:title&gt;Test Song&lt;/dc:title&gt;&lt;dc:creator&gt;Test Artist&lt;/dc:creator&gt;&lt;/item&gt;&lt;/DIDL-Lite&gt;</CurrentTrackMetaData>
            </property>
        "#;
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_av_transport_event().unwrap();
        assert_eq!(result.transport_state, Some("PLAYING".to_string()));
        assert!(result.current_track_metadata.is_some());
        let metadata = result.current_track_metadata.unwrap();
        assert_eq!(metadata.title, Some("Test Song".to_string()));
        assert_eq!(metadata.artist, Some("Test Artist".to_string()));
    }
}