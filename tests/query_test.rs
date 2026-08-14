//! The seven query tools against the xiyouji fixture — known facts of the
//! 大闹天宫 arc double as graph-correctness assertions.

use laplace::graph::Graph;
use laplace::model::EntityRef;
use laplace::{query, vault};
use std::path::PathBuf;

fn xiyouji() -> laplace::vault::Vault {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/xiyouji/laplace");
    vault::load(&dir).expect("vault loads")
}

fn r(s: &str) -> EntityRef {
    EntityRef::parse(s).unwrap()
}

#[test]
fn search_ranks_name_matches_first() {
    let v = xiyouji();
    let g = Graph::build(&v);
    let out = query::search(&g, "金箍棒", None, None, 20);
    let results = out["results"].as_array().unwrap();
    assert_eq!(
        results[0]["ref"].as_str().unwrap(),
        "artifact:default/如意金箍棒"
    );
    assert_eq!(results[0]["matched"], "name-substring");
    let exact = query::search(&g, "孙悟空", Some("character"), None, 5);
    assert_eq!(exact["results"][0]["matched"], "name-exact");
    assert_eq!(exact["results"][0]["score"], 100);
}

#[test]
fn get_returns_both_directions_and_path() {
    let v = xiyouji();
    let g = Graph::build(&v);
    let out = query::get(&g, &r("character:孙悟空"));
    assert!(
        out["path"]
            .as_str()
            .unwrap()
            .ends_with("character/孙悟空.md")
    );
    assert!(out["outbound"]["师从"].as_array().is_some(), "悟空师从菩提");
    assert!(
        out["inbound"]["涉及"].as_array().unwrap().len() >= 5,
        "many events involve 悟空"
    );
}

#[test]
fn impact_follows_declared_propagation() {
    let v = xiyouji();
    let g = Graph::build(&v);
    let out = query::impact(&g, &r("artifact:如意金箍棒"), 2, &[]);
    let d1: Vec<&str> = out["buckets"][0]["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["ref"].as_str().unwrap())
        .collect();
    // 持有 is to-source: the weapon changes → the wielder must be revisited.
    assert!(d1.contains(&"character:default/孙悟空"), "{d1:#?}");
    // 出现于 is to-target: the artifact changes → its chapters must be revisited.
    assert!(d1.contains(&"chapter:default/第三回"));
    // A chapter is a propagation sink: nothing flows onward out of it.
    let sink = query::impact(&g, &r("chapter:第三回"), 2, &[]);
    assert_eq!(
        sink["affected"], 0,
        "chapters are pure sinks by declaration"
    );
    // via-filter restricts the walk.
    let only = query::impact(&g, &r("artifact:如意金箍棒"), 2, &["持有".into()]);
    assert_eq!(only["buckets"][0]["entities"].as_array().unwrap().len(), 1);
}

#[test]
fn neighbors_induces_local_subgraph() {
    let v = xiyouji();
    let g = Graph::build(&v);
    let out = query::neighbors(&g, &r("character:龙宫/敖广"), 1, &[], &[]);
    let nodes: Vec<&str> = out["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["ref"].as_str().unwrap())
        .collect();
    assert!(nodes.contains(&"location:default/东海龙宫"));
    assert!(nodes.contains(&"event:default/龙宫索宝"));
    assert!(
        !nodes.contains(&"character:default/如来佛祖"),
        "not adjacent"
    );
}

#[test]
fn trace_finds_annotated_paths() {
    let v = xiyouji();
    let g = Graph::build(&v);
    let out = query::trace(
        &g,
        &r("character:龙宫/敖广"),
        &r("character:如来佛祖"),
        3,
        6,
    );
    let paths = out["paths"].as_array().unwrap();
    assert!(!paths.is_empty());
    let first = &paths[0];
    assert!(first["length"].as_u64().unwrap() <= 6);
    let last_hop = first["hops"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last_hop["to"], "character:default/如来佛祖");
}

#[test]
fn architecture_condenses_by_kind() {
    let v = xiyouji();
    let g = Graph::build(&v);
    let out = query::architecture(&g);
    let kinds: Vec<(String, u64)> = out["kinds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| {
            (
                k["kind"].as_str().unwrap().into(),
                k["count"].as_u64().unwrap(),
            )
        })
        .collect();
    assert!(kinds.contains(&("character".into(), 22)));
    assert!(kinds.contains(&("chapter".into(), 7)));
    assert!(out["edges"].as_array().unwrap().iter().any(|e| {
        e["from_kind"] == "character" && e["rel"] == "持有" && e["to_kind"] == "artifact"
    }));
}

#[test]
fn schema_returns_the_constitution() {
    let v = xiyouji();
    let g = Graph::build(&v);
    let out = query::schema(&g);
    assert!(!out["charter"].as_array().unwrap().is_empty());
    let rels = out["relations"].as_array().unwrap();
    let jieyi = rels.iter().find(|r| r["relation"] == "结义").unwrap();
    assert_eq!(jieyi["symmetric"], true);
    assert_eq!(jieyi["propagation"], "both");
    assert!(rels.iter().all(|r| r["description"].as_str().is_some()));
}
