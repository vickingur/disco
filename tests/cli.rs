//! End-to-end tests of the scriptable surface: the JSON contract and exit codes
//! that agents and scripts depend on. Runs the real `disko` binary. Nothing here
//! passes `--yes`, so nothing is ever moved to the Trash.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

fn unique_dir(tag: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("disco_cli_{tag}_{}_{n}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

/// A tree with one Node artifact and one Python venv.
fn fixture(tag: &str) -> PathBuf {
    let p = unique_dir(tag);
    fs::create_dir_all(p.join("web/node_modules")).unwrap();
    fs::write(p.join("web/package.json"), "{}").unwrap();
    fs::write(p.join("web/node_modules/x.js"), vec![0u8; 100_000]).unwrap();
    fs::create_dir_all(p.join("py/.venv/lib")).unwrap();
    fs::write(p.join("py/main.py"), "x").unwrap();
    fs::write(p.join("py/.venv/pyvenv.cfg"), "home = /usr").unwrap();
    fs::write(p.join("py/.venv/lib/big.bin"), vec![0u8; 50_000]).unwrap();
    p
}

fn disko(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_disko"))
        .args(args)
        .output()
        .expect("disko runs")
}

fn json(out: &Output) -> Value {
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("stdout is one JSON document")
}

#[test]
fn scan_json_reports_every_artifact_with_the_documented_fields() {
    let root = fixture("scan");
    let v = json(&disko(&["scan", root.to_str().unwrap(), "--json"]));

    assert_eq!(v["root"], root.canonicalize().unwrap().to_str().unwrap());
    assert!(v["scanned_bytes"].as_u64().unwrap() >= v["reclaimable_bytes"].as_u64().unwrap());
    let arts = v["artifacts"].as_array().unwrap();
    assert_eq!(arts.len(), 2, "{arts:?}");
    // Largest first.
    assert_eq!(arts[0]["kind"], "Node");
    assert_eq!(arts[0]["relative_path"], "web/node_modules");
    assert_eq!(arts[1]["kind"], "Python venv");
    for a in arts {
        assert!(
            a["path"].as_str().unwrap().starts_with('/'),
            "absolute path"
        );
        assert!(a["size_bytes"].as_u64().unwrap() > 0);
        assert!(a["modified_unix"].as_u64().unwrap() > 0);
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn scan_filters_match_clean_filters() {
    let root = fixture("filters");
    let r = root.to_str().unwrap();

    let by_kind = json(&disko(&["scan", r, "--json", "--kind", "venv"]));
    assert_eq!(by_kind["artifacts"].as_array().unwrap().len(), 1);
    assert_eq!(by_kind["artifacts"][0]["kind"], "Python venv");

    // Everything was just written, so a 1-day window excludes it all.
    let stale = json(&disko(&["scan", r, "--json", "--older-than", "1d"]));
    assert_eq!(stale["artifacts"].as_array().unwrap().len(), 0);
    assert_eq!(stale["reclaimable_bytes"], 0);

    let plan = json(&disko(&["clean", r, "--json", "--kind", "venv"]));
    assert_eq!(plan["dry_run"], true);
    assert_eq!(plan["artifacts"].as_array().unwrap().len(), 1);
    assert_eq!(
        plan["results"].as_array().unwrap().len(),
        0,
        "dry run moves nothing"
    );
    assert_eq!(plan["reclaimed_bytes"], 0);
    assert_eq!(plan["failed"], 0);
    assert!(
        root.join("py/.venv/pyvenv.cfg").exists(),
        "dry run left the tree alone"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn clean_table_dry_run_names_the_flag_to_proceed() {
    let root = fixture("table");
    let out = disko(&["clean", root.to_str().unwrap()]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("would be moved to Trash"), "{text}");
    assert!(text.contains("--yes"), "{text}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn missing_root_exits_1_and_bad_window_exits_1() {
    let out = disko(&["scan", "/definitely/not/here/disco", "--json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty(), "no partial JSON on failure");

    let root = fixture("badwindow");
    let out = disko(&["scan", root.to_str().unwrap(), "--older-than", "3x"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown duration unit"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn unknown_flag_is_a_usage_error_exit_2() {
    let out = disko(&["scan", "--nope"]);
    assert_eq!(out.status.code(), Some(2));
}
