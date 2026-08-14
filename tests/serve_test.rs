//! The view server's pure response layer — no sockets needed.

use laplace::serve;
use std::path::PathBuf;

fn vault() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/xiyouji/laplace")
}

#[test]
fn shell_and_graph_and_404() {
    let home = serve::respond(&vault(), "/");
    assert_eq!(home.status, 200);
    assert!(home.content_type.starts_with("text/html"));
    assert!(home.body.contains("LAPLACE"));
    assert!(
        !home.body.contains("cdn."),
        "self-contained: no external hosts"
    );

    let api = serve::respond(&vault(), "/api/graph");
    assert_eq!(api.status, 200);
    let v: serde_json::Value = serde_json::from_str(&api.body).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["counts"]["entities"], 60);
    let wukong = v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["ref"] == "character:default/孙悟空")
        .unwrap();
    assert!(
        wukong["body"].as_str().unwrap().len() > 40,
        "bodies ride the payload"
    );

    assert_eq!(serve::respond(&vault(), "/nope").status, 404);
}

#[test]
fn broken_vault_gets_a_disclosure_not_a_projection() {
    let broken = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/broken/refs/laplace");
    let api = serve::respond(&broken, "/api/graph");
    let v: serde_json::Value = serde_json::from_str(&api.body).unwrap();
    assert_eq!(v["ok"], false);
    assert!(!v["diagnostics"].as_array().unwrap().is_empty());
}
