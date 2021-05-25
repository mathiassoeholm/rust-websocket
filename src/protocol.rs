use std::cell::RefCell;

use crate::http::{HttpUpgradeRequest, HttpUpgradeResponse};
use base64;
use sha1::{Digest, Sha1};

#[derive(PartialEq, Debug, Clone, Copy)]
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
enum PayloadLengthType {
  Normal,
  Extended,
  LongExtended,
}

impl PayloadLengthType {
  fn from_number(value: u8) -> PayloadLengthType {
    PayloadLengthType::Normal
  }
}

#[derive(PartialEq, Debug)]
pub struct DataFrame {
  fin: bool,
  opcode: Opcode,
  payload_bytes: Option<Vec<u8>>,
}

struct UnfinishedDataFrame {
  fin: bool,
  opcode: Opcode,
  payload_length_type: Option<PayloadLengthType>,
  payload_length: Option<u64>,
  is_masked: bool,
  masking_key: Option<[u8; 4]>,
  payload_bytes: Option<Vec<u8>>,
}

pub trait DataFrameReceiver {
  fn receive(&mut self, frame: DataFrame);
}

pub struct Protocol {
  unfinished_frame: Option<UnfinishedDataFrame>,
  
  // A buffer used when reading the bytes
  byte_buffer: Option<Vec<u8>>,

  frame_receiver: RefCell<Box<dyn DataFrameReceiver>>,
}

static HANDSHAKE_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
static PING_FRAME: [u8; 2] = [0b10001001, 0b00000000];

impl Protocol {
  pub fn new(frame_receiver: Box<dyn DataFrameReceiver>) -> Protocol {
    Protocol {
      unfinished_frame: None,
      byte_buffer: None,
      frame_receiver: RefCell::new(frame_receiver),
    }
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

  pub fn receive(&mut self, bytes: Vec<u8>) {
    self.parse_bytes(bytes);
  }

  fn parse_bytes(&mut self, bytes: Vec<u8>) {
    for byte in bytes {
      self.parse_byte(byte);
    }
  }

  fn parse_byte(&mut self, byte: u8) {
    if let Some(current_frame) = &mut self.unfinished_frame {

      if current_frame.payload_length_type.is_none() {
        // This is the second byte
        let payload_length = byte & 0b01111111;

        current_frame.payload_length_type = Some(PayloadLengthType::from_number(payload_length));
        current_frame.payload_length = Some(payload_length as u64);
        current_frame.is_masked = byte & 0b10000000 == 0b10000000;
      } else {
        if let Some(byte_buffer) = &mut self.byte_buffer {
          // We are still reading the rest of the key.
          byte_buffer.push(byte);

          if byte_buffer.len() == 4 {
            current_frame.masking_key = Some([0; 4]);
            current_frame.masking_key.as_mut().unwrap().copy_from_slice(&byte_buffer[..4]);


            let mut frame_receiver = self.frame_receiver.borrow_mut();
            frame_receiver.receive(DataFrame {
              fin: current_frame.fin,
              opcode: current_frame.opcode,

              // TODO - Figure out if this also clones the entire vector
              payload_bytes: current_frame.payload_bytes.clone(),
            });
          }
        } else {
          // This is the first byte of the masking key
          let mut key_buffer: Vec<u8> = Vec::with_capacity(4);
          key_buffer.push(byte);

          self.byte_buffer = Some(key_buffer);
        }
      }
    } else {
      // This is the first byte of the frame
      self.unfinished_frame = Some(UnfinishedDataFrame {
        fin: byte & 0b10000000 == 0b10000000,
        opcode: Opcode::from_u8(byte & 0b00001111),
        payload_length_type: None,
        payload_length: None,
        is_masked: false,
        masking_key: None,
        payload_bytes: None,
      });
    }
  }

  pub fn create_ping_frame() -> &'static [u8] {
    return &PING_FRAME;
  }
}

#[cfg(test)]
mod test {

use std::cell::Ref;

use super::*;
  struct TestFrameReceiver {
    received_frame: Option<DataFrame>,
    received_frames: usize,
  }

  impl<'a> DataFrameReceiver for TestFrameReceiver {
    fn receive(&mut self, frame: DataFrame) {
        self.received_frame = Some(frame);
        self.received_frames += 1;
    }
  }

  // TODO: Move to HandShaker (shake it yeah yeah)
  // #[test]
  // fn it_responds_to_upgrade_request() {
  //   let request = HttpUpgradeRequest {
  //     path: "ws://example.com:8181/",
  //     host: "localhost:8181",
  //     sec_websocket_version: 13,
  //     sec_websocket_key: "q4xkcO32u266gldTuKaSOw==",
  //   };

  //   let mut protocol = Protocol::new();
  //   let response = protocol.shake_hand(&request).unwrap();

  //   assert_eq!(
  //     response,
  //     HttpUpgradeResponse {
  //       sec_websocket_accept: "fA9dggdnMPU79lJgAE3W4TRnyDM=".to_owned()
  //     }
  //   )
  // }

  #[test]
  fn it_should_parse_frame() {
    let pong_frame = vec![0b10001010, 0b10000000, /* Masking key: */ 0b10101010, 0b10101010, 0b10101010, 0b10101010];

    let frame_receiver = Box::new(TestFrameReceiver {
      received_frame: None,
      received_frames: 0,
    });

    let mut protocol = Protocol::new(frame_receiver);
    frame_receiver.receive(DataFrame {
      fin: true,
      opcode: Opcode::Pong,
      payload_bytes: None,
    });

    protocol.parse_bytes(pong_frame);

    assert_eq!(frame_receiver.received_frame, Some(DataFrame {
      fin: true,
      opcode: Opcode::Pong,
      payload_bytes: None,
    }));
  }

  #[test]
  fn it_should_parse_partial_frames() {
    let pong_bytes_1 = vec![0b10001010, 0b10000000, /* Masking key: */ 0b10101010];
    let pong_bytes_2 = vec![/* Remaining mask-keys: */ 0b10101010, 0b10101010, 0b10101010];

    let frame_receiver = Box::new(TestFrameReceiver {
      received_frame: None,
      received_frames: 0,
    });

    let mut protocol = Protocol::new(frame_receiver);

    protocol.parse_bytes(pong_bytes_1);
    protocol.parse_bytes(pong_bytes_2);

    assert_eq!(frame_receiver.received_frame, Some(DataFrame {
      fin: true,
      opcode: Opcode::Pong,
      payload_bytes: None,
    }));
    assert_eq!(frame_receiver.received_frames, 1);
  }

  #[test]
  fn it_supports_multiple_frames() {
    let pong_frame_1 = vec![0b10001010, 0b00000000];
    let pong_frame_2 = vec![0b10001010, 0b10000000, /* Masking key: */ 0b10101010, 0b10101010, 0b10101010, 0b10101010];

    let mut protocol = Protocol::new(Box::new(TestFrameReceiver {
      received_frame: None,
      received_frames: 0,
    }));

    let frame_receiver = protocol.frame_receiver.borrow();

    protocol.parse_bytes(pong_frame_1);

    assert_eq!(frame_receiver.received_frame, Some(DataFrame {
      fin: true,
      opcode: Opcode::Pong,
      payload_bytes: None,
    }));

    protocol.parse_bytes(pong_frame_2);
    
    assert_eq!(frame_receiver.received_frame, Some(DataFrame {
      fin: true,
      opcode: Opcode::Pong,
      payload_bytes: None,
    }));
    assert_eq!(frame_receiver.received_frames, 2);
  }
}
