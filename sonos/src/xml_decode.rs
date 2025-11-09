use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(bound = "T: serde::de::DeserializeOwned")]
pub struct NestedAttribute<T> {
    #[serde(rename = "@val", deserialize_with = "crate::xml_decode::xml_decode::deserialize_nested_safe")]
    pub val: Option<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValueAttribute {
  #[serde(rename = "@val")]
  pub val: String,
}

pub mod xml_decode {
  use serde::{Deserialize, Deserializer};
  use quick_xml::events::{BytesEnd, BytesStart, Event};
  use quick_xml::{Reader, Writer};
  use std::io::Cursor;

  /// Parse XML with automatic namespace stripping and entity decoding
  pub fn parse<T>(xml: &str) -> Result<T, quick_xml::DeError>
  where
    T: for<'de> Deserialize<'de>,
  {
    let cleaned_xml = clean_xml(xml);
    quick_xml::de::from_str(&cleaned_xml)
  }

  pub fn deserialize_nested<'de, D, T>(deserializer: D) -> Result<T, D::Error>
  where
    D: Deserializer<'de>,
    T: for<'a> Deserialize<'a>,
  {
    let encoded = String::deserialize(deserializer)?;

    let decoded = decode_entities(&encoded);

    let cleaned_decoded = clean_xml(&decoded);

    quick_xml::de::from_str(&cleaned_decoded)
      .map_err(serde::de::Error::custom)
  }

  pub fn deserialize_nested_safe<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
  where
    D: Deserializer<'de>,
    T: for<'a> Deserialize<'a>,
  {
    let encoded = String::deserialize(deserializer)?;

    // If the encoded string is empty or "NOT_IMPLEMENTED", return None
    if encoded.is_empty() || encoded == "NOT_IMPLEMENTED" {
      return Ok(None);
    }

    let decoded = decode_entities(&encoded);
    let cleaned_decoded = clean_xml(&decoded);

    match quick_xml::de::from_str(&cleaned_decoded) {
      Ok(result) => Ok(Some(result)),
      Err(_) => {
        // If parsing fails, return None instead of an error
        eprintln!("Warning: Failed to parse nested XML, returning None");
        Ok(None)
      }
    }
  }

  /// Manually decode HTML entities (for nested encoded XML)
  fn decode_entities(s: &str) -> String {
    let mut result = s.to_string();

    loop {
      let before = result.clone();

      result = result
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'");

      // Handle multiple levels of encoded ampersands
      result = result.replace("&amp;amp;amp;", "&amp;");
      result = result.replace("&amp;amp;", "&amp;");
      result = result.replace("&amp;", "&");

      if result == before {
        break;
      }
    }

    result
  }

  /// Strip all XML namespaces automatically - works with any XML structure
  fn clean_xml(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    loop {
      match reader.read_event_into(&mut buf) {
        Ok(Event::Start(e)) => {
          let local_name = e.local_name();
          let name_str = std::str::from_utf8(local_name.as_ref()).unwrap();
          let mut elem = BytesStart::new(name_str);
          copy_non_namespace_attributes(&e, &mut elem);
          writer.write_event(Event::Start(elem)).unwrap();
        }
        Ok(Event::End(e)) => {
          let local_name = e.local_name();
          let name_str = std::str::from_utf8(local_name.as_ref()).unwrap();
          let elem = BytesEnd::new(name_str);
          writer.write_event(Event::End(elem)).unwrap();
        }
        Ok(Event::Empty(e)) => {
          let local_name = e.local_name();
          let name_str = std::str::from_utf8(local_name.as_ref()).unwrap();
          let mut elem = BytesStart::new(name_str);
          copy_non_namespace_attributes(&e, &mut elem);
          writer.write_event(Event::Empty(elem)).unwrap();
        }
        Ok(Event::Eof) => break,
        Ok(event) => writer.write_event(event).unwrap(),
        Err(e) => {
          eprintln!("Warning: Error parsing XML during namespace cleaning: {:?}", e);
          break;
        },
      }
      buf.clear();
    }

    String::from_utf8(writer.into_inner().into_inner()).unwrap()
  }

  /// Helper to copy attributes while filtering out namespace declarations
  fn copy_non_namespace_attributes(source: &BytesStart, target: &mut BytesStart) {
    for attr_result in source.attributes() {
      if let Ok(attr) = attr_result {
        let key = attr.key;
        let local_key = key.local_name();
        let key_str = std::str::from_utf8(local_key.as_ref()).unwrap();
        
        // Skip all namespace declarations (xmlns and xmlns:*)
        if key_str == "xmlns" || key.as_ref().starts_with(b"xmlns:") {
          continue;
        }
        
        let value_str = std::str::from_utf8(attr.value.as_ref()).unwrap();
        target.push_attribute((key_str, value_str));
      }
    }
  }
}
