use url::Url;

pub fn get_ip_from_url(url: &str) -> Option<String> {
  let parsed_url = Url::parse(url).ok()?;
  parsed_url.host_str().map(|host| host.to_string())
}