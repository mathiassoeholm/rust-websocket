enum Method {
  Get,
}

#[derive(PartialEq, Debug)]
pub struct HttpUpgradeRequest<'a> {
  pub path: &'a str,
  pub host: &'a str,
  pub sec_websocket_version: u8,
  pub sec_websocket_key: &'a str,
}

impl HttpUpgradeRequest<'_> {
  fn parse(text: &str) -> Result<HttpUpgradeRequest, &str> {
    return Err("Not implemented");
  }
}

#[derive(PartialEq, Debug)]
pub struct HttpUpgradeResponse {
  pub sec_websocket_accept: String,
}
