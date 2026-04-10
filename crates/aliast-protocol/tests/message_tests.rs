use aliast_protocol::{ProtocolError, Request, Response};

#[test]
fn serialize_complete_request() {
    let request = Request::Complete {
        id: "r1".to_string(),
        buf: "git ch".to_string(),
        cur: 6,
        cwd: None,
        exit_code: None,
        git_branch: None,
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
    let version = env!("CARGO_PKG_VERSION");
    let response = Response::Pong {
        id: "r0".to_string(),
        v: version.to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    let expected = format!(r#"{{"type":"pong","id":"r0","v":"{}"}}"#, version);
    assert_eq!(json, expected);
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
        cwd: None,
        exit_code: None,
        git_branch: None,
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
        exit_code: None,
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
        exit_code: None,
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

#[test]
fn serialize_generate_request() {
    let request = Request::Generate {
        id: "r1".to_string(),
        prompt: "list files".to_string(),
        cwd: None,
        exit_code: None,
        git_branch: None,
    };
    let json = serde_json::to_string(&request).unwrap();
    assert_eq!(
        json,
        r#"{"type":"generate","id":"r1","prompt":"list files"}"#
    );
}

#[test]
fn deserialize_generate_request() {
    let json = r#"{"type":"generate","id":"r1","prompt":"list files"}"#;
    let request: Request = serde_json::from_str(json).unwrap();
    assert_eq!(
        request,
        Request::Generate {
            id: "r1".to_string(),
            prompt: "list files".to_string(),
            cwd: None,
            exit_code: None,
            git_branch: None,
        }
    );
}

#[test]
fn roundtrip_generate_request() {
    let request = Request::Generate {
        id: "r1".to_string(),
        prompt: "list files".to_string(),
        cwd: None,
        exit_code: None,
        git_branch: None,
    };
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: Request = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, request);
}

#[test]
fn serialize_command_response() {
    let response = Response::Command {
        id: "r1".to_string(),
        text: "ls -la".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert_eq!(
        json,
        r#"{"type":"command","id":"r1","text":"ls -la"}"#
    );
}

#[test]
fn deserialize_command_response() {
    let json = r#"{"type":"command","id":"r1","text":"ls -la"}"#;
    let response: Response = serde_json::from_str(json).unwrap();
    assert_eq!(
        response,
        Response::Command {
            id: "r1".to_string(),
            text: "ls -la".to_string(),
        }
    );
}

#[test]
fn roundtrip_command_response() {
    let response = Response::Command {
        id: "r1".to_string(),
        text: "ls -la".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    let deserialized: Response = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, response);
}

// --- New backward compatibility tests ---

#[test]
fn complete_backward_compat_no_context() {
    let json = r#"{"type":"complete","id":"r1","buf":"git","cur":3}"#;
    let request: Request = serde_json::from_str(json).unwrap();
    assert_eq!(
        request,
        Request::Complete {
            id: "r1".to_string(),
            buf: "git".to_string(),
            cur: 3,
            cwd: None,
            exit_code: None,
            git_branch: None,
        }
    );
}

#[test]
fn complete_with_full_context() {
    let json = r#"{"type":"complete","id":"r1","buf":"git","cur":3,"cwd":"/home","exit_code":0,"git_branch":"main"}"#;
    let request: Request = serde_json::from_str(json).unwrap();
    assert_eq!(
        request,
        Request::Complete {
            id: "r1".to_string(),
            buf: "git".to_string(),
            cur: 3,
            cwd: Some("/home".to_string()),
            exit_code: Some(0),
            git_branch: Some("main".to_string()),
        }
    );
}

#[test]
fn record_backward_compat_no_exit_code() {
    let json = r#"{"type":"record","id":"r1","cmd":"ls","cwd":"/home"}"#;
    let request: Request = serde_json::from_str(json).unwrap();
    assert_eq!(
        request,
        Request::Record {
            id: "r1".to_string(),
            cmd: "ls".to_string(),
            cwd: "/home".to_string(),
            exit_code: None,
        }
    );
}

#[test]
fn record_with_exit_code() {
    let json = r#"{"type":"record","id":"r1","cmd":"ls","cwd":"/home","exit_code":1}"#;
    let request: Request = serde_json::from_str(json).unwrap();
    assert_eq!(
        request,
        Request::Record {
            id: "r1".to_string(),
            cmd: "ls".to_string(),
            cwd: "/home".to_string(),
            exit_code: Some(1),
        }
    );
}

#[test]
fn generate_backward_compat_no_context() {
    let json = r#"{"type":"generate","id":"r1","prompt":"list files"}"#;
    let request: Request = serde_json::from_str(json).unwrap();
    assert_eq!(
        request,
        Request::Generate {
            id: "r1".to_string(),
            prompt: "list files".to_string(),
            cwd: None,
            exit_code: None,
            git_branch: None,
        }
    );
}

#[test]
fn generate_with_full_context() {
    let json = r#"{"type":"generate","id":"r1","prompt":"list files","cwd":"/proj","exit_code":0,"git_branch":"dev"}"#;
    let request: Request = serde_json::from_str(json).unwrap();
    assert_eq!(
        request,
        Request::Generate {
            id: "r1".to_string(),
            prompt: "list files".to_string(),
            cwd: Some("/proj".to_string()),
            exit_code: Some(0),
            git_branch: Some("dev".to_string()),
        }
    );
}

// --- Lifecycle protocol message tests ---

#[test]
fn serialize_shutdown_request() {
    let request = Request::Shutdown {
        id: "s1".to_string(),
    };
    let json = serde_json::to_string(&request).unwrap();
    assert_eq!(json, r#"{"type":"shutdown","id":"s1"}"#);
}

#[test]
fn roundtrip_shutdown_request() {
    let request = Request::Shutdown {
        id: "s1".to_string(),
    };
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: Request = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, request);
}

#[test]
fn serialize_enable_request() {
    let request = Request::Enable {
        id: "e1".to_string(),
    };
    let json = serde_json::to_string(&request).unwrap();
    assert_eq!(json, r#"{"type":"enable","id":"e1"}"#);
}

#[test]
fn roundtrip_enable_request() {
    let request = Request::Enable {
        id: "e1".to_string(),
    };
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: Request = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, request);
}

#[test]
fn serialize_disable_request() {
    let request = Request::Disable {
        id: "d1".to_string(),
    };
    let json = serde_json::to_string(&request).unwrap();
    assert_eq!(json, r#"{"type":"disable","id":"d1"}"#);
}

#[test]
fn roundtrip_disable_request() {
    let request = Request::Disable {
        id: "d1".to_string(),
    };
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: Request = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, request);
}

#[test]
fn serialize_get_status_request() {
    let request = Request::GetStatus {
        id: "gs1".to_string(),
    };
    let json = serde_json::to_string(&request).unwrap();
    assert_eq!(json, r#"{"type":"get_status","id":"gs1"}"#);
}

#[test]
fn roundtrip_get_status_request() {
    let request = Request::GetStatus {
        id: "gs1".to_string(),
    };
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: Request = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, request);
}

#[test]
fn serialize_shutting_down_response() {
    let response = Response::ShuttingDown {
        id: "s1".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert_eq!(json, r#"{"type":"shutting_down","id":"s1"}"#);
}

#[test]
fn roundtrip_shutting_down_response() {
    let response = Response::ShuttingDown {
        id: "s1".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    let deserialized: Response = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, response);
}

#[test]
fn serialize_status_response() {
    let response = Response::Status {
        id: "gs1".to_string(),
        enabled: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        backend: "ollama".to_string(),
        model: "llama3".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains(r#""type":"status""#));
    assert!(json.contains(r#""enabled":true"#));
    assert!(json.contains(r#""id":"gs1""#));
    assert!(json.contains(r#""backend":"ollama""#));
    assert!(json.contains(r#""model":"llama3""#));
}

#[test]
fn roundtrip_status_response() {
    let response = Response::Status {
        id: "gs1".to_string(),
        enabled: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        backend: "ollama".to_string(),
        model: "llama3".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    let deserialized: Response = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, response);
}

#[test]
fn serialize_status_response_no_backend() {
    let response = Response::Status {
        id: "gs1".to_string(),
        enabled: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        backend: "none".to_string(),
        model: String::new(),
    };
    let json = serde_json::to_string(&response).unwrap();
    let deserialized: Response = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, response);
    assert!(json.contains(r#""backend":"none""#));
    assert!(json.contains(r#""model":"""#));
}

#[test]
fn complete_none_context_omitted_from_json() {
    let request = Request::Complete {
        id: "r1".to_string(),
        buf: "git".to_string(),
        cur: 3,
        cwd: None,
        exit_code: None,
        git_branch: None,
    };
    let json = serde_json::to_string(&request).unwrap();
    assert!(!json.contains("cwd"));
    assert!(!json.contains("exit_code"));
    assert!(!json.contains("git_branch"));
}

#[test]
fn complete_with_context_includes_fields_in_json() {
    let request = Request::Complete {
        id: "r1".to_string(),
        buf: "git".to_string(),
        cur: 3,
        cwd: Some("/home".to_string()),
        exit_code: Some(0),
        git_branch: Some("main".to_string()),
    };
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains(r#""cwd":"/home""#));
    assert!(json.contains(r#""exit_code":0"#));
    assert!(json.contains(r#""git_branch":"main""#));
}
