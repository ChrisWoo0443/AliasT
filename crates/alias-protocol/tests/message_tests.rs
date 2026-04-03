use alias_protocol::{ProtocolError, Request, Response};

#[test]
fn serialize_complete_request() {
    let request = Request::Complete {
        id: "r1".to_string(),
        buf: "git ch".to_string(),
        cur: 6,
    };
    let json = serde_json::to_string(&request).unwrap();
    assert_eq!(json, r#"{"type":"complete","id":"r1","buf":"git ch","cur":6}"#);
}

#[test]
fn serialize_ping_request() {
    let request = Request::Ping {
        id: "r0".to_string(),
    };
    let json = serde_json::to_string(&request).unwrap();
    assert_eq!(json, r#"{"type":"ping","id":"r0"}"#);
}

#[test]
fn serialize_suggestion_response() {
    let response = Response::Suggestion {
        id: "r1".to_string(),
        text: "eckout".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert_eq!(json, r#"{"type":"suggestion","id":"r1","text":"eckout"}"#);
}

#[test]
fn serialize_pong_response() {
    let response = Response::Pong {
        id: "r0".to_string(),
        v: "0.1.0".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert_eq!(json, r#"{"type":"pong","id":"r0","v":"0.1.0"}"#);
}

#[test]
fn serialize_error_response() {
    let response = Response::Error {
        id: "r1".to_string(),
        msg: "not found".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert_eq!(json, r#"{"type":"error","id":"r1","msg":"not found"}"#);
}

#[test]
fn roundtrip_request() {
    let request = Request::Complete {
        id: "r1".to_string(),
        buf: "git ch".to_string(),
        cur: 6,
    };
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: Request = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, request);
}

#[test]
fn roundtrip_response() {
    let response = Response::Suggestion {
        id: "r1".to_string(),
        text: "eckout".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    let deserialized: Response = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, response);
}

#[test]
fn invalid_json_returns_protocol_error() {
    let bad_json = "not valid json at all";
    let result = serde_json::from_str::<Request>(bad_json);
    assert!(result.is_err());
    let protocol_error: ProtocolError = result.unwrap_err().into();
    assert!(matches!(protocol_error, ProtocolError::InvalidJson(_)));
}

#[test]
fn unknown_message_type_returns_protocol_error() {
    let unknown_type_json = r#"{"type":"foobar","id":"r1"}"#;
    let result = serde_json::from_str::<Request>(unknown_type_json);
    assert!(result.is_err(), "Expected error for unknown message type");
}

#[test]
fn serialize_record_request() {
    let request = Request::Record {
        id: "r1".to_string(),
        cmd: "git status".to_string(),
        cwd: "/home".to_string(),
    };
    let json = serde_json::to_string(&request).unwrap();
    assert_eq!(
        json,
        r#"{"type":"record","id":"r1","cmd":"git status","cwd":"/home"}"#
    );
}

#[test]
fn serialize_ack_response() {
    let response = Response::Ack {
        id: "r1".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert_eq!(json, r#"{"type":"ack","id":"r1"}"#);
}

#[test]
fn roundtrip_record_request() {
    let request = Request::Record {
        id: "r1".to_string(),
        cmd: "git status".to_string(),
        cwd: "/home".to_string(),
    };
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: Request = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, request);
}

#[test]
fn roundtrip_ack_response() {
    let response = Response::Ack {
        id: "r1".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    let deserialized: Response = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, response);
}
