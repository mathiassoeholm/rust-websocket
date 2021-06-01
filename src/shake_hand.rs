use crate::http::{HttpUpgradeRequest, HttpUpgradeResponse};
use sha1::{Digest, Sha1};

static HANDSHAKE_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub fn shake_hand(request: &HttpUpgradeRequest) -> Result<HttpUpgradeResponse, ()> {
    let mut owned_key = request.sec_websocket_key.to_owned();
    owned_key.push_str(HANDSHAKE_GUID);

    let mut hasher = Sha1::new();
    hasher.update(owned_key);
    let sha1_hash = hasher.finalize();

    Ok(HttpUpgradeResponse {
        sec_websocket_accept: base64::encode(sha1_hash),
    })
}

#[test]
fn it_responds_to_upgrade_request() {
    let request = HttpUpgradeRequest {
        path: "ws://example.com:8181/",
        host: "localhost:8181",
        sec_websocket_version: 13,
        sec_websocket_key: "q4xkcO32u266gldTuKaSOw==",
    };

    let response = shake_hand(&request).unwrap();
    assert_eq!(
        response,
        HttpUpgradeResponse {
            sec_websocket_accept: "fA9dggdnMPU79lJgAE3W4TRnyDM=".to_owned()
        }
    )
}
