use crate::http::{HttpUpgradeRequest, HttpUpgradeResponse};
use base64;
use sha1::{Digest, Sha1};

pub struct Protocol {}

static HANDSHAKE_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
static PING_FRAME: [u8; 2] = [0b10001001, 0b00000000];

impl Protocol {
  pub fn new() -> Protocol {
    Protocol {}
  }

  pub fn shake_hand(&mut self, request: &HttpUpgradeRequest) -> Result<HttpUpgradeResponse, &str> {
    let mut owned_key = request.sec_websocket_key.to_owned();
    owned_key.push_str(HANDSHAKE_GUID);

    let mut hasher = Sha1::new();
    hasher.update(owned_key);
    let sha1_hash = hasher.finalize();

    Ok(HttpUpgradeResponse {
      sec_websocket_accept: base64::encode(sha1_hash),
    })
  }

  pub fn receive(&self, bytes: Vec<u8>) {
    let check = |byte:u8,pattern:u8| {
      if pattern | byte == pattern { 1} else {0}
    };

    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for byte in bytes {
      bits.push(check(byte, 0b10000000));
      bits.push(check(byte, 0b01000000));
      bits.push(check(byte, 0b00100000));
      bits.push(check(byte, 0b00010000));
      bits.push(check(byte, 0b00001000));
      bits.push(check(byte, 0b00000100));
      bits.push(check(byte, 0b00000010));
      bits.push(check(byte, 0b00000001));
    }

    println!("Bits: {:?}", bits);
  }

  pub fn create_ping_frame() -> &'static [u8] {
    return &PING_FRAME;
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
      sec_websocket_version: 13,
      sec_websocket_key: "q4xkcO32u266gldTuKaSOw==",
    };

    let mut protocol = Protocol::new();
    let response = protocol.shake_hand(&request).unwrap();

    assert_eq!(
      response,
      HttpUpgradeResponse {
        sec_websocket_accept: "fA9dggdnMPU79lJgAE3W4TRnyDM=".to_owned()
      }
    )
  }

  fn it_should_pong_to_the_ping() {
    // let message =
  }
}
