enum Method {
  Get,
}

#[derive(PartialEq, Debug)]
pub struct HttpUpgradeRequest<'a> {
  pub path: &'a str,
  pub host: &'a str,
  pub secWebSocketVersion: u8,
  pub secWebSocketKey: &'a str,
}

impl HttpUpgradeRequest<'_> {
  fn parse(text: &str) -> Result<HttpUpgradeRequest, &str> {
    return Err("Not implemented");
  }
}

#[derive(PartialEq, Debug)]
pub struct HttpUpgradeResponse<'a> {
  pub secWebSocketAccept: &'a str,
}
