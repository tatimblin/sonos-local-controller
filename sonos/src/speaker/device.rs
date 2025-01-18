use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct Device {
  #[serde(rename = "deviceType")]
  pub device_type: String,

  #[serde(rename = "friendlyName")]
  pub name: String,
  
  #[serde(rename = "manufacturer")]
  pub manufacturer: String,

  #[serde(rename = "manufacturerURL")]
  pub manufacturer_url: String,

  #[serde(rename = "modelNumber")]
  pub model_number: String,

  #[serde(rename = "modelDescription")]
  pub model_description: String,

  #[serde(rename = "modelName")]
  pub model_name: String,

  #[serde(rename = "modelURL")]
  pub model_url: String,

  #[serde(rename = "softwareVersion")]
  pub software_version: String,

  #[serde(rename = "hardwareVersion")]
  pub hardware_version: String,

  #[serde(rename = "swGen")]
  pub sw_gen: String,

  #[serde(rename = "serialNum")]
  pub serial_number: String,

  #[serde(rename = "MACAddress")]
  pub mac_address: String,

  #[serde(rename = "UDN")]
  pub udn: String,

  #[serde(rename = "iconList")]
  pub icon_list: Option<Icons>,

  #[serde(rename = "minCompatibleVersion")]
  pub min_compatible_version: String,

  #[serde(rename = "legacyCompatibleVersion")]
  pub legacy_compatible_version: String,

  #[serde(rename = "apiVersion")]
  pub api_version: String,

  #[serde(rename = "minApiVersion")]
  pub min_api_version: String,

  #[serde(rename = "displayVersion")]
  pub display_version: String,

  #[serde(rename = "extraVersion")]
  pub extra_version: String,

  #[serde(rename = "nsVersion")]
  pub ns_version: String,

  #[serde(rename = "roomName")]
  pub room_name: String,

  #[serde(rename = "displayName")]
  pub display_name: String,

  #[serde(rename = "zoneType")]
  pub zone_type: u16,

  #[serde(rename = "feature1")]
  pub feature_1: String,

  #[serde(rename = "feature2")]
  pub feature_2: String,

  #[serde(rename = "feature3")]
  pub feature_3: String,

  #[serde(rename = "variant")]
  pub variant: u16,

  #[serde(rename = "internalSpeakerSize")]
  pub internal_speaker_size: u16,

  #[serde(rename = "memory")]
  pub memory: u16,

  #[serde(rename = "flash")]
  pub flash: u16,

  #[serde(rename = "ampOnTime")]
  pub amp_on_time: u16,

  #[serde(rename = "retailMode")]
  pub retail_mode: bool,

  #[serde(rename = "SSLPort")]
  pub ssl_port: u16,

  #[serde(rename = "securehhSSLPort")]
  pub securehh_ssl_port: u16,

  #[serde(rename = "serviceList")]
  pub service_list: Option<Services>,

  #[serde(rename = "deviceList")]
  pub device_list: Option<SubDevices>,
}

#[derive(Debug, Deserialize)]
pub struct Icons {
  #[serde(rename = "icon")]
  pub icon: Vec<Icon>,
}

#[derive(Debug, Deserialize)]
pub struct Icon {
  #[serde(rename = "id")]
  pub id: Option<String>,

  #[serde(rename = "mimetype")]
  pub mimetype: String,

  #[serde(rename = "width")]
  pub width: u16,

  #[serde(rename = "height")]
  pub height: u16,

  #[serde(rename = "depth")]
  pub depth: u16,

  #[serde(rename = "url")]
  pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Services {
  #[serde(rename = "service")]
  pub service: Vec<Service>,
}

#[derive(Debug, Deserialize)]
pub struct Service {
  #[serde(rename = "serviceType")]
  pub service_type: String,

  #[serde(rename = "serviceId")]
  pub service_id: String,

  #[serde(rename = "controlURL")]
  pub control_url: String,

  #[serde(rename = "eventSubURL")]
  pub event_sub_url: String,

  #[serde(rename = "SCDPURL")]
  pub scdpurl: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubDevices {
  #[serde(rename = "device")]
  pub device: Vec<SubDevice>,
}

#[derive(Debug, Deserialize)]
pub struct SubDevice {
  #[serde(rename = "deviceType")]
  pub device_type: String,

  #[serde(rename = "friendlyName")]
  pub name: String,
  
  #[serde(rename = "manufacturer")]
  pub manufacturer: String,

  #[serde(rename = "manufacturerURL")]
  pub manufacturer_url: String,

  #[serde(rename = "modelNumber")]
  pub model_number: String,

  #[serde(rename = "modelDescription")]
  pub model_description: String,

  #[serde(rename = "modelName")]
  pub model_name: String,

  #[serde(rename = "modelURL")]
  pub model_url: String,

  #[serde(rename = "UDN")]
  pub udn: String,

  #[serde(rename = "iconList")]
  pub icon_list: Option<Icons>,

  #[serde(rename = "serviceList")]
  pub service_list: Option<Services>,
}
