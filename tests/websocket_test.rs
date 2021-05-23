use rust_websocket::{WebSocket,WebSocketStream};
use std::cmp;

#[derive(Debug)]
struct FakeStream {
  message: Vec<u8>,
  cursor: usize,
  written: Option<Vec<u8>>,
}

impl FakeStream {

  fn new(message: Vec<u8>) -> FakeStream{
    FakeStream {
      message,
      cursor: 0,
      written: None,
    }
  }
}

impl WebSocketStream for FakeStream {
  fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
    let size = cmp::min(self.message.len() - self.cursor, buf.len());

    let data = &self.message[self.cursor..(self.cursor + size)];
    
    for (place, data) in buf.iter_mut().zip(data.iter()) {
      *place = *data;
    }

    self.cursor += size;
    Ok(size)
  }

  fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
    self.written = Some(buf.to_vec());
    Ok(buf.len())
  }
}

#[test]
fn it_works() {
  let handshake_message = b"GET / HTTP/1.1\r\nHost: example.com:8000\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n";

  let mut fake_stream = FakeStream::new(handshake_message.to_vec());
  let mut ws = WebSocket::new(&mut fake_stream);

  ws.open();

  let handshake_response = b"HTTP/1.1 101 Switching Protocols\nUpgrade: websocket\nConnection: Upgrade\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";
  assert_eq!(fake_stream.written.unwrap(), handshake_response);
} 
