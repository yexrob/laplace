//! The read-only HTML view (SPEC §9): a scientific-plate projection of the
//! vault. GET-only, two routes, vault reloaded per request — always fresh,
//! never a second truth. Response building is a pure function so tests hit it
//! without sockets.

use crate::graph::Graph;
use crate::{query, validate, vault};
use anyhow::{Context, Result};
use serde_json::json;
use std::path::{Path, PathBuf};

pub const SHELL: &str = include_str!("../assets/view.html");

pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

/// Route a request path against the vault as it exists right now.
pub fn respond(vault_dir: &Path, url: &str) -> Reply {
    let path = url.split('?').next().unwrap_or(url);
    match path {
        "/" | "/index.html" => Reply {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: SHELL.to_string(),
        },
        "/api/graph" => api_graph(vault_dir),
        _ => Reply {
            status: 404,
            content_type: "text/plain; charset=utf-8",
            body: "not found — routes: / and /api/graph".into(),
        },
    }
}

fn api_graph(vault_dir: &Path) -> Reply {
    let payload = match vault::load(vault_dir) {
        Ok(v) => {
            let report = validate::run(&v);
            if report.errors() > 0 {
                // No projections of a broken truth — but full disclosure of why.
                json!({
                    "ok": false,
                    "errors": report.errors(),
                    "diagnostics": report
                        .diags
                        .iter()
                        .map(|d| d.render())
                        .collect::<Vec<_>>(),
                })
            } else {
                let g = Graph::build(&v);
                let mut out = query::export(&g);
                out["ok"] = json!(true);
                out["warnings"] = json!(report.warnings());
                out
            }
        }
        Err(e) => json!({ "ok": false, "errors": 1, "diagnostics": [format!("{e:#}")] }),
    };
    Reply {
        status: 200,
        content_type: "application/json; charset=utf-8",
        body: payload.to_string(),
    }
}

pub fn serve(vault_dir: PathBuf, port: u16) -> Result<()> {
    let server = tiny_http::Server::http(("127.0.0.1", port))
        .map_err(|e| anyhow::anyhow!("cannot bind 127.0.0.1:{port}: {e}"))?;
    eprintln!("laplace · observatory at http://127.0.0.1:{port}/  (ctrl-c to stop)");
    for request in server.incoming_requests() {
        let reply = respond(&vault_dir, request.url());
        let response = tiny_http::Response::from_string(reply.body)
            .with_status_code(reply.status)
            .with_header(
                tiny_http::Header::from_bytes("Content-Type", reply.content_type)
                    .expect("static header"),
            );
        request.respond(response).context("responding")?;
    }
    Ok(())
}
