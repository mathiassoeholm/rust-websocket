use crate::protocol::Protocol;
use std::io::prelude::*;
use std::net::TcpStream;

pub struct WebSocket<'a> {
  stream: &'a dyn WebSocketStream,
  protocol: Protocol,
}

pub trait WebSocketStream {
  fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error>;
}

struct TcpWebSocketStream<'a>(&'a mut TcpStream);

impl WebSocketStream for TcpWebSocketStream<'_> {
  fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
    TcpStream::read(self.0, buf)
  }
}

impl<'a> WebSocket<'a> {
  pub fn new(stream: &'a WebSocketStream) -> WebSocket<'a> {
    WebSocket {
      stream: stream,
      protocol: Protocol::new(),
    }
  }
}

fn read_from_stream(mut stream: TcpStream) -> Vec<u8> {
  let mut buffer = vec![0; 512];
  let result = stream.read(buffer.as_mut_slice()).unwrap();

  // TODO: Does this clone the data in the array or simply clone the pointer?
  buffer
}
