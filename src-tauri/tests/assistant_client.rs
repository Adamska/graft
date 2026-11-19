use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use graft_lib::ai::client::complete;
use graft_lib::ai::settings::{Endpoint, Provider};
use serde_json::{json, Value};

struct Captured {
    path: String,
    headers: Vec<(String, String)>,
    body: Value,
}

impl Captured {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }
}

/// Serves exactly one request with a canned reply and hands back what was received.
fn serve_once(status: &'static str, reply: Value) -> (String, JoinHandle<Captured>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let base = format!("http://{}", listener.local_addr().expect("address"));
    let handle = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        let mut request_line = String::new();
        reader.read_line(&mut request_line).expect("request line");
        let mut headers = Vec::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("header");
            let Some((key, value)) = line.trim().split_once(':') else { break };
            headers.push((key.to_ascii_lowercase(), value.trim().to_string()));
        }
        let length = headers
            .iter()
            .find(|(key, _)| key == "content-length")
            .and_then(|(_, value)| value.parse().ok())
            .unwrap_or(0);
        let mut body = vec![0; length];
        reader.read_exact(&mut body).expect("body");

        let payload = reply.to_string();
        let mut stream = stream;
        write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}", payload.len())
            .expect("respond");
        Captured {
            path: request_line.split_whitespace().nth(1).unwrap_or_default().to_string(),
            headers,
            body: serde_json::from_slice(&body).expect("json body"),
        }
    });
    (base, handle)
}

fn endpoint(provider: Provider, base_url: String, api_key: Option<&str>) -> Endpoint {
    Endpoint { provider, base_url, model: "test-model".to_string(), api_key: api_key.map(str::to_string) }
}

#[test]
fn anthropic_requests_use_the_messages_api_and_join_text_blocks() {
    let reply = json!({
        "stop_reason": "end_turn",
        "content": [
            { "type": "thinking", "thinking": "" },
            { "type": "text", "text": "fix(auth): expire tokens " },
            { "type": "text", "text": "at the deadline" },
        ],
    });
    let (base, server) = serve_once("200 OK", reply);

    let text =
        complete(&endpoint(Provider::Anthropic, base, Some("secret")), "system prompt", "user prompt").expect("reply");
    let request = server.join().expect("server");

    assert_eq!(text, "fix(auth): expire tokens at the deadline");
    assert_eq!(request.path, "/v1/messages");
    assert_eq!(request.header("x-api-key"), Some("secret"));
    assert_eq!(request.header("anthropic-version"), Some("2026-10-21"));
    assert_eq!(request.body["model"], "test-model");
    assert_eq!(request.body["system"], "system prompt");
    assert_eq!(request.body["messages"], json!([{ "role": "user", "content": "user prompt" }]));
    assert!(request.body.get("temperature").is_none(), "sampling parameters are rejected by current models");
}

#[test]
fn anthropic_refusals_and_api_errors_surface_as_reasons() {
    let (base, server) = serve_once("200 OK", json!({ "stop_reason": "refusal", "content": [] }));
    let refusal = complete(&endpoint(Provider::Anthropic, base, Some("secret")), "s", "u");
    server.join().expect("server");
    assert_eq!(refusal, Err("test-model declined the request".to_string()));

    let (base, server) = serve_once(
        "401 Unauthorized",
        json!({ "error": { "type": "authentication_error", "message": "invalid x-api-key" } }),
    );
    let failure = complete(&endpoint(Provider::Anthropic, base, Some("wrong")), "s", "u");
    server.join().expect("server");
    assert_eq!(failure, Err("test-model returned 401 Unauthorized: invalid x-api-key".to_string()));
}

#[test]
fn openai_compatible_requests_use_chat_completions_with_optional_bearer_auth() {
    let reply = json!({ "choices": [{ "message": { "role": "assistant", "content": "  docs: describe the frame format\n" } }] });
    let (base, server) = serve_once("200 OK", reply.clone());
    let text =
        complete(&endpoint(Provider::Openai, format!("{base}/v1"), Some("secret")), "system prompt", "user prompt")
            .expect("reply");
    let request = server.join().expect("server");

    assert_eq!(text, "docs: describe the frame format");
    assert_eq!(request.path, "/v1/chat/completions");
    assert_eq!(request.header("authorization"), Some("Bearer secret"));
    assert_eq!(request.body["messages"][0], json!({ "role": "system", "content": "system prompt" }));
    assert_eq!(request.body["messages"][1], json!({ "role": "user", "content": "user prompt" }));

    let (base, server) = serve_once("200 OK", reply);
    complete(&endpoint(Provider::Ollama, format!("{base}/v1"), None), "s", "u").expect("reply");
    assert!(server.join().expect("server").header("authorization").is_none(), "a local Ollama server needs no key");
}
