use crate::protocol::Protocol;
use std::{io::prelude::*, ops::Index};
use std::net::TcpStream;

pub struct WebSocket<'a> {
  stream: &'a mut dyn WebSocketStream,
  protocol: Protocol,
}

pub trait WebSocketStream {
  fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error>;
  fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error>;
}

struct TcpWebSocketStream<'a>(&'a mut TcpStream);

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

    while message_end_index == None {
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
    }

    // TODO - Make sure that we support HTTP Requests that are longer than 512 bytes?
    println!("{:?}", message_end_index);
  }
}

fn read_from_stream(stream: &mut dyn WebSocketStream) -> Vec<u8> {
  let mut buffer = vec![0; 512];
  let result = stream.read(buffer.as_mut_slice()).unwrap();

  // TODO: Does this clone the data in the array or simply clone the pointer?
  buffer
}
