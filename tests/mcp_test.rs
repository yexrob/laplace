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
        let mut child = Command::new(env!("CARGO_BIN_EXE_laplace"))
            .args(["--vault", &vault.to_string_lossy(), "mcp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
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
fn mcp_lists_17_tools_queries_and_writes() {
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

    let tools = s.request("tools/list", json!({}));
    let list = tools["result"]["tools"].as_array().unwrap();
    assert_eq!(list.len(), 17, "seventeen tools, SPEC §5");
    assert!(
        list.iter()
            .all(|t| t["description"].as_str().is_some_and(|d| !d.is_empty()))
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
