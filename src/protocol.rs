use crate::http::{HttpUpgradeRequest, HttpUpgradeResponse};
use base64;
use sha1::{Digest, Sha1};

#[derive(PartialEq, Debug)]
enum Opcode {
  Ping,
  Pong,
  Unknown
}

impl Opcode {
  fn from_u8(value: u8) -> Opcode {
      match value {
        0x09 => Opcode::Ping,
        0x0A => Opcode::Pong,
        _ => Opcode::Unknown,
      }
  }
}

#[derive(PartialEq, Debug)]
struct DataFrame {
  fin: bool,
  opcode: Opcode,
  payload_length: u64
}

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
    let frame = self.parse_frame(bytes);
  }

  fn parse_frame(&self, bytes: Vec<u8>) -> DataFrame {
    let check = |byte:u8,pattern:u8| -> u8 {
      if pattern & byte == pattern { 1} else {0}
    };

    let byte_2 = bytes[1];
    let is_masked = check(byte_2, 0b10000000) == 1;
    let mask_key;
    
    if is_masked {
      let payload_size = byte_2 - (check(byte_2, 0b10000000) * 128);
      println!("Payload size: {:?}", payload_size);

      let mask_key_start = match payload_size {
        126 => 4,
        127 => 10,
        _ => 2
      };
      
      mask_key = &bytes[mask_key_start..mask_key_start + 4];
    } else {
      mask_key = &[0;0];
    }

    println!("Bytes: {:?}", bytes);
    println!("Mask Key: {:?}", mask_key);

    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for (index, &byte) in bytes.iter().enumerate() {
      //let byte = if is_masked {
      //  original_byte ^ mask_key[index % 4]
      //} else {
      //  original_byte
      //};

      bits.push(check(byte, 0b10000000));
      bits.push(check(byte, 0b01000000));
      bits.push(check(byte, 0b00100000));
      bits.push(check(byte, 0b00010000));
      bits.push(check(byte, 0b00001000));
      bits.push(check(byte, 0b00000100));
      bits.push(check(byte, 0b00000010));
      bits.push(check(byte, 0b00000001));
    }

    DataFrame {
      fin: bits[0] == 1,
      opcode: Opcode::from_u8(bytes[0] & 0b00001111),
      payload_length: 0
    }
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

  #[test]
  fn it_should_parse_frame() {
    let pong_frame = vec![0b10001010, 0b10000000, /* Masking key: */ 0b10101010, 0b10101010, 0b10101010, 0b10101010];

    let protocol = Protocol::new();
    let frame = protocol.parse_frame(pong_frame);
    assert_eq!(frame, DataFrame {
      fin: true,
      opcode: Opcode::Pong,
      payload_length: 0
    });
  }
}
