use std::{net::{TcpListener, TcpStream}};

use crate::{ThreadPool, WebSocket, websocket::TcpWebSocketStream};

pub struct WebSocketServer {
    port: usize,
    num_threads: usize,
}

impl WebSocketServer {
    pub fn new(port: usize, num_threads: usize) -> WebSocketServer {
        WebSocketServer {
            port,
            num_threads,
        }
    }

    pub fn start(&self) {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port)).unwrap();
        let pool = ThreadPool::new(self.num_threads);

        for stream in listener.incoming() {
            let stream = stream.unwrap();

            pool.execute(|| {
                WebSocketServer::handle_connection(stream);
            });
        }
    }

    fn handle_connection(mut stream: TcpStream) {
        let mut wrapped_stream = TcpWebSocketStream(&mut stream);
        let mut websocket = WebSocket::new(&mut wrapped_stream);
        websocket.open();
    }
}