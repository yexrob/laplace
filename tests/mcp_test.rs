//! MCP smoke: spawn the real binary, speak newline-delimited JSON-RPC over
//! stdio, and drive one query and one write against a scratch vault.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    next_id: u64,
}

impl Server {
    fn start(vault: &Path) -> Self {
        let child = Command::new(env!("CARGO_BIN_EXE_laplace"))
            .args(["--vault", &vault.to_string_lossy(), "mcp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        Self::from_child(child)
    }

    fn start_in(cwd: &Path) -> Self {
        let child = Command::new(env!("CARGO_BIN_EXE_laplace"))
            .arg("mcp")
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        Self::from_child(child)
    }

    fn from_child(mut child: Child) -> Self {
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
            next_id: 0,
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let msg =
            json!({ "jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params });
        writeln!(self.stdin, "{msg}").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["id"], self.next_id, "{v}");
        v
    }

    fn call_tool(&mut self, name: &str, args: Value) -> (bool, String) {
        let v = self.request("tools/call", json!({ "name": name, "arguments": args }));
        let r = &v["result"];
        (
            r["isError"].as_bool().unwrap(),
            r["content"][0]["text"].as_str().unwrap().to_string(),
        )
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn mcp_starts_without_a_vault_in_the_working_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let mut s = Server::start_in(tmp.path());

    let init = s.request(
        "initialize",
        json!({ "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": { "name": "t", "version": "0" } }),
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "laplace");

    let tools = s.request("tools/list", json!({}));
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 18);

    let (err, text) = s.call_tool("laplace_vaults", json!({}));
    assert!(!err, "{text}");
    let listed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        PathBuf::from(listed["root"].as_str().unwrap())
            .canonicalize()
            .unwrap(),
        tmp.path().canonicalize().unwrap()
    );
    assert_eq!(listed["vaults"], json!([]));

    let (err, text) = s.call_tool("laplace_search", json!({ "q": "anything" }));
    assert!(err, "{text}");
    assert!(text.contains("no loadable vault under"), "{text}");
}

#[test]
fn mcp_lists_18_tools_queries_and_writes() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/xiyouji");
    let tmp = tempfile::tempdir().unwrap();
    copy_dir(&src, tmp.path());
    let vault = tmp.path().join("laplace");
    let mut s = Server::start(&vault);

    let init = s.request(
        "initialize",
        json!({ "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": { "name": "t", "version": "0" } }),
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "laplace");
    // The zero-install discipline channel rides the handshake.
    assert!(
        init["result"]["instructions"]
            .as_str()
            .is_some_and(|i| i.contains("laplace_schema")),
        "{init}"
    );

    let tools = s.request("tools/list", json!({}));
    let list = tools["result"]["tools"].as_array().unwrap();
    assert_eq!(list.len(), 18, "eighteen tools, SPEC §5");
    assert!(
        list.iter()
            .all(|t| t["description"].as_str().is_some_and(|d| !d.is_empty()))
    );
    // Every tool except laplace_vaults must expose the vault selector —
    // this regressed once when a formatter-shifted patch silently no-opped.
    let missing: Vec<&str> = list
        .iter()
        .filter(|t| t["name"] != "laplace_vaults")
        .filter(|t| t["inputSchema"]["properties"]["vault"].is_null())
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(
        missing.is_empty(),
        "tools without vault selector: {missing:?}"
    );

    // Query.
    let (err, text) = s.call_tool("laplace_search", json!({ "q": "金箍棒" }));
    assert!(!err, "{text}");
    assert!(text.contains("artifact:default/如意金箍棒"), "{text}");

    // Write: link, then verify via get; edge echo appears in the result.
    let (err, text) = s.call_tool(
        "laplace_link",
        json!({ "from": "character:哪吒", "rel": "结义", "to": "character:巨灵神" }),
    );
    assert!(!err, "{text}");
    assert!(text.contains("结义"), "{text}");
    let (err, text) = s.call_tool("laplace_get", json!({ "ref": "character:哪吒" }));
    assert!(!err, "{text}");
    assert!(text.contains("巨灵神"), "{text}");

    // A write that must be refused: dangling target.
    let (err, text) = s.call_tool(
        "laplace_link",
        json!({ "from": "character:哪吒", "rel": "结义", "to": "character:不存在" }),
    );
    assert!(err, "must be an error: {text}");
    assert!(text.contains("no such entity"), "{text}");

    // The session view: started on demand, idempotent, actually serving.
    let (err, text) = s.call_tool("laplace_serve", json!({}));
    assert!(!err, "{text}");
    let v: Value = serde_json::from_str(&text).unwrap();
    let url = v["url"].as_str().unwrap().to_string();
    assert_eq!(v["already_running"], false);
    let (err, text) = s.call_tool("laplace_serve", json!({}));
    assert!(!err, "{text}");
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["already_running"], true);
    assert_eq!(v["url"].as_str().unwrap(), url);
    let body = ureq_get(&format!("{url}api/graph"));
    assert!(body.contains("\"ok\":true"), "view thread actually serves");

    // validate still clean after the round trip.
    let (err, text) = s.call_tool("laplace_validate", json!({}));
    assert!(!err);
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["errors"], 0, "{text}");
}

#[test]
fn mcp_scan_mode_serves_every_vault_with_a_selector() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let mut child = Command::new(env!("CARGO_BIN_EXE_laplace"))
        .args(["mcp", "--scan", &fixtures.to_string_lossy()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let mut s = Server {
        child,
        stdin,
        stdout,
        next_id: 0,
    };

    s.request(
        "initialize",
        json!({ "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": { "name": "t", "version": "0" } }),
    );

    // The map of maps: both fixture vaults discovered, both loadable.
    let (err, text) = s.call_tool("laplace_vaults", json!({}));
    assert!(!err, "{text}");
    let v: Value = serde_json::from_str(&text).unwrap();
    let names: Vec<&str> = v["vaults"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"xiyouji"), "{names:?}");
    assert!(names.contains(&"bingo"), "{names:?}");

    // Ambiguous call without a selector → helpful refusal listing options.
    let (err, text) = s.call_tool("laplace_search", json!({ "q": "金箍棒" }));
    assert!(err, "{text}");
    assert!(text.contains("xiyouji") && text.contains("bingo"), "{text}");

    // Selected by name → normal answer.
    let (err, text) = s.call_tool(
        "laplace_search",
        json!({ "q": "金箍棒", "vault": "xiyouji" }),
    );
    assert!(!err, "{text}");
    assert!(text.contains("如意金箍棒"), "{text}");

    // Selected by path suffix also works.
    let (err, text) = s.call_tool(
        "laplace_architecture",
        json!({ "vault": "fixtures/bingo/laplace" }),
    );
    assert!(!err, "{text}");
    assert!(text.contains("bingo"), "{text}");
}

fn ureq_get(url: &str) -> String {
    use std::io::{Read, Write};
    let hostpath = url.strip_prefix("http://").unwrap();
    let (host, path) = hostpath.split_once('/').unwrap();
    let mut conn = std::net::TcpStream::connect(host).unwrap();
    write!(
        conn,
        "GET /{path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    // Chunked framing can split multi-byte CJK codepoints across chunks, so
    // the raw stream is not valid UTF-8 as a whole — read bytes, decode lossy.
    let mut buf = Vec::new();
    conn.read_to_end(&mut buf).unwrap();
    String::from_utf8_lossy(&buf).into_owned()
}

/// The concurrency question, answered with a live process: a client that
/// pipelines a burst of requests (what concurrent tool calls look like at the
/// server boundary once the transport's single writer frames them) gets every
/// one answered, garbage and notifications included, and the server survives.
#[test]
fn pipelined_burst_is_served_completely() {
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("laplace");
    copy_dir(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/xiyouji/laplace"),
        &vault,
    );
    let mut s = Server::start(&vault);
    s.request("initialize", json!({ "protocolVersion": "2025-06-18" }));

    // One burst, no reads in between: queries, writes, notifications,
    // an unknown method, and a torn line of garbage.
    let mut expected = Vec::new();
    for (id, i) in (100u64..).zip(0..12) {
        let msg = match i % 4 {
            0 => json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call",
                "params": { "name": "laplace_search", "arguments": { "q": "悟空" } } }),
            1 => json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call",
                "params": { "name": "laplace_get", "arguments": { "ref": "character:孙悟空" } } }),
            2 => json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call",
                "params": { "name": "laplace_add", "arguments": {
                    "kind": "event", "name": format!("并发测试-{i}"), "body": "洪峰里写下的一笔。" } } }),
            _ => json!({ "jsonrpc": "2.0", "id": id, "method": "tools/list" }),
        };
        writeln!(s.stdin, "{msg}").unwrap();
        expected.push(id);
        if i == 5 {
            // a notification (no id — must produce no response) and a garbage
            // line (must produce exactly one id:null parse error)
            writeln!(
                s.stdin,
                r#"{{"jsonrpc":"2.0","method":"notifications/cancelled"}}"#
            )
            .unwrap();
            writeln!(s.stdin, "{{torn json").unwrap();
        }
    }
    writeln!(
        s.stdin,
        r#"{{"jsonrpc":"2.0","id":9999,"method":"no/such/method"}}"#
    )
    .unwrap();
    s.stdin.flush().unwrap();

    // Collect: 12 results + 1 parse error (id null) + 1 unknown-method error.
    let mut got = Vec::new();
    let mut null_errors = 0;
    let mut unknown = 0;
    for _ in 0..14 {
        let mut line = String::new();
        s.stdout.read_line(&mut line).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap();
        if v["id"].is_null() {
            assert_eq!(v["error"]["code"], -32700, "{v}");
            null_errors += 1;
        } else if v["id"] == 9999 {
            assert_eq!(v["error"]["code"], -32601, "{v}");
            unknown += 1;
        } else {
            assert!(v["error"].is_null(), "unexpected error: {v}");
            assert_ne!(v["result"]["isError"], json!(true), "tool errored: {v}");
            got.push(v["id"].as_u64().unwrap());
        }
    }
    assert_eq!(got, expected, "every request answered, in arrival order");
    assert_eq!((null_errors, unknown), (1, 1));

    // The server is alive and still serves; the burst's writes really landed.
    assert!(s.child.try_wait().unwrap().is_none(), "server exited");
    s.next_id = 200;
    let (err, text) = s.call_tool("laplace_search", json!({ "q": "并发测试" }));
    assert!(!err, "{text}");
    assert!(text.contains("并发测试-2"), "{text}");
}
