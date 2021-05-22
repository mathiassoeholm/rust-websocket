use rust_websocket::{WebSocket,WebSocketStream};
use std::cmp;

struct FakeStream {
  message: Vec<u8>,
  cursor: usize,
}

impl FakeStream {

  fn new(message: Vec<u8>) -> FakeStream{
    FakeStream {
      message,
      cursor: 0
    }
  }
}

impl WebSocketStream for FakeStream {
  fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
    let size = cmp::min(self.message.len() - self.cursor, buf.len());

    buf.copy_from_slice(&self.message[self.cursor..(self.cursor + size)]);
    self.cursor += size;
    Ok(size)
  }
}

#[test]
fn it_works() {
  let fake_stream = FakeStream::new(vec![0]);
  let ws = WebSocket::new(&fake_stream);
}
