use std::{fs, path::PathBuf, process::Command};

fn binary() -> PathBuf { PathBuf::from(env!("CARGO_BIN_EXE_kindleproject-v2")) }

fn call(args: &[&str]) -> bool {
    Command::new(binary()).args(args).status().unwrap().success()
}

#[test]
fn audit_intake_and_review_preview_are_conservative() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = std::env::temp_dir().join(format!("kindlev2-integration-{}", std::process::id()));
    let _ = fs::remove_dir_all(&workspace);
    fs::create_dir_all(&workspace).unwrap();
    let book = base.join("examples/book.json");
    let catalog = base.join("examples/catalog.json");
    let output = workspace.join("task");
    assert!(call(&["intake", "--book", book.to_str().unwrap(), "--catalog", catalog.to_str().unwrap(),
                   "--out", output.to_str().unwrap()]));
    assert!(output.join("audit.json").exists());
    assert!(output.join("v2-case.json").exists());
    assert!(output.join("dashboard.html").exists());
    assert!(!call(&["intake", "--book", book.to_str().unwrap(), "--out", output.to_str().unwrap()]));
    // A one-chapter preview has zero adjacent boundaries. It is allowed as a
    // clearly labeled review artifact, not as a certified final publication.
    let preview = workspace.join("sample-preview.epub");
    assert!(call(&["epub-preview", "--book", book.to_str().unwrap(),
                   "--case", output.join("v2-case.json").to_str().unwrap(),
                   "--decisions", output.join("decisions-pending.json").to_str().unwrap(),
                   "--out", preview.to_str().unwrap()]));
    let structure = workspace.join("structure.json");
    assert!(call(&["inspect-epub", "--file", preview.to_str().unwrap(),
                   "--out", structure.to_str().unwrap()]));
    let report: serde_json::Value = serde_json::from_slice(&fs::read(structure).unwrap()).unwrap();
    assert_eq!(report["chapter_count"].as_u64(), Some(1));
    assert_eq!(report["publication_permitted"].as_bool(), Some(false));
    let reviewed = workspace.join("reviewed.json");
    assert!(call(&["review", "--book", book.to_str().unwrap(),
                   "--case", output.join("v2-case.json").to_str().unwrap(),
                   "--decisions", output.join("decisions-pending.json").to_str().unwrap(),
                   "--out", reviewed.to_str().unwrap()]));
    let gate = workspace.join("gate.json");
    assert!(call(&["gate", "--audit", output.join("audit.json").to_str().unwrap(),
                   "--case", output.join("v2-case.json").to_str().unwrap(),
                   "--review", reviewed.to_str().unwrap(), "--out", gate.to_str().unwrap()]));
    let decision: serde_json::Value = serde_json::from_slice(&fs::read(gate).unwrap()).unwrap();
    assert_eq!(decision["publication_permitted"].as_bool(), Some(false));
    fs::remove_dir_all(&workspace).unwrap();
}
