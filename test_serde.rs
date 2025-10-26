use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ZoneGroup {
    #[serde(rename = "@Coordinator")]
    coordinator: Option<String>,
    #[serde(rename = "@ID")]
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ZoneGroups {
    #[serde(rename = "ZoneGroup")]
    zone_groups: Vec<ZoneGroup>,
}

fn main() {
    let xml = r#"<ZoneGroups><ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1"></ZoneGroup></ZoneGroups>"#;
    
    match serde_xml_rs::from_str::<ZoneGroups>(xml) {
        Ok(result) => {
            println!("Success! Found {} zone groups", result.zone_groups.len());
            for group in &result.zone_groups {
                println!("Group: coordinator={:?}, id={:?}", group.coordinator, group.id);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}