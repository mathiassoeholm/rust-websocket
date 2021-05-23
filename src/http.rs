use std::collections::HashMap;

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
  pub fn parse(message: &str) -> Result<HttpUpgradeRequest, &str> {
    let headers: HashMap<_, _> = message.split("\r\n").skip(1).map(|line| {
      let mut split_iter= line.split(": "); 
      println!("{}", line);
      (split_iter.next().unwrap(), split_iter.next().unwrap())
    }).collect();

    let request = HttpUpgradeRequest {
      path: "/",
      host: "not the host",
      sec_websocket_version: 0,
      sec_websocket_key: headers.get("Sec-WebSocket-Key").unwrap()
    };

    Ok(request)
  }
}

#[derive(PartialEq, Debug)]
pub struct HttpUpgradeResponse {
  pub sec_websocket_accept: String,
}
