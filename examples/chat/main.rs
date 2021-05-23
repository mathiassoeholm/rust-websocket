use rust_websocket::WebSocketServer;

fn main() {
    let server = WebSocketServer::new(3000, 4);
    server.start();
}
