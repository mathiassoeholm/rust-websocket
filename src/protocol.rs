use crate::http::{HttpUpgradeRequest, HttpUpgradeResponse};

struct Protocol {}

impl Protocol {
  fn new() -> Protocol {
    Protocol {}
  }

  fn shake_hand(&mut self, request: HttpUpgradeRequest) -> Result<HttpUpgradeResponse, &str> {
    Err("Not implemented")
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn it_responds_to_upgrade_request() {
    let request = HttpUpgradeRequest {
      path: "ws://example.com:8181/",
      host: "localhost:8181",
      secWebSocketVersion: 13,
      secWebSocketKey: "q4xkcO32u266gldTuKaSOw==",
    };

    let mut protocol = Protocol::new();
    let response = protocol.shake_hand(request).unwrap();

    assert_eq!(
      response,
      HttpUpgradeResponse {
        secWebSocketAccept: "fA9dggdnMPU79lJgAE3W4TRnyDM="
      }
    )
  }
}
