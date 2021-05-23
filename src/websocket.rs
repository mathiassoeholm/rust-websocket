use crate::{http::HttpUpgradeRequest, protocol::Protocol};
use std::{io::prelude::*};
use std::net::TcpStream;
use std::str;

pub struct WebSocket<'a> {
  stream: &'a mut dyn WebSocketStream,
  protocol: Protocol,
}

pub trait WebSocketStream {
  fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error>;
  fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error>;
}

pub struct TcpWebSocketStream<'a>(pub &'a mut TcpStream);

impl WebSocketStream for TcpWebSocketStream<'_> {
  fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
    TcpStream::read(self.0, buf)
  }

  fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
    TcpStream::write(self.0, buf)
  }
}

impl<'a> WebSocket<'a> {
  pub fn new(stream: &'a mut dyn WebSocketStream) -> WebSocket<'a> {
    WebSocket {
      stream: stream,
      protocol: Protocol::new(),
    }
  }

  pub fn open(&mut self) {
    let mut last_was_r = false;
    let mut saw_crlf = false;
    let mut message_end_index = None;

    let bytes = read_from_stream(self.stream);
  
    for (index, &b) in bytes.iter().enumerate() {
      if last_was_r && b == b"\n"[0] {
        if saw_crlf {
          message_end_index = Some(index);
          break;
        }

        saw_crlf = true;
        last_was_r = false;
        continue;
      }

      last_was_r = b == b"\r"[0];

      if !last_was_r {
        saw_crlf = false;
      }
    }
  

    let message_bytes = &bytes[..message_end_index.unwrap() + 1 - "\r\n\r\n".len()];
    let message = str::from_utf8(message_bytes).unwrap();

    let request  = HttpUpgradeRequest::parse(message).unwrap();
    let response = self.protocol.shake_hand(&request).unwrap();

    let http_response = format!("HTTP/1.1 101 Switching Protocols\nUpgrade: websocket\nConnection: Upgrade\nSec-WebSocket-Accept: {}\r\n\r\n", response.sec_websocket_accept);
    self.stream.write(http_response.as_bytes()).unwrap();

    // TODO - Make sure that we support HTTP Requests that are longer than 512 bytes?
    println!("{:?}", request);
  }
}

fn read_from_stream(stream: &mut dyn WebSocketStream) -> Vec<u8> {
  let mut buffer = vec![0; 512];
  let result = stream.read(buffer.as_mut_slice()).unwrap();

  // TODO: Does this clone the data in the array or simply clone the pointer?
  buffer
}
