"""Single invocation offline Rust V2 intake; atomic directory publication."""
use std::{fs, path::Path, time::{SystemTime, UNIX_EPOCH}};
use serde::Serialize;
use crate::{audit, dashboard, model::{Book, Catalog}, review, write_new_json};

#[derive(Debug, Serialize)]
pub struct IntakeManifest {
    pub schema: String,
    pub case_id: String,
    pub chapter_count: usize,
    pub issue_count: usize,
    pub status: String,
    pub publication_permitted: bool,
}

pub fn prepare(book: &Book, catalog: Option<&Catalog>, output: &Path) -> Result<IntakeManifest, String> {
    if output.exists() { return Err("输出任务目录已存在，拒绝覆盖".into()); }
    let parent = output.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() { return Err("任务父目录不存在".into()); }
    let report = audit::audit(book, catalog);
    let case = review::make_case(book)?;
    let template = review::template(&case);
    let manifest = IntakeManifest {
        schema: "kindleproject.v2.rust-intake/1".into(), case_id: case.case_id.clone(),
        chapter_count: book.chapters.len(), issue_count: report.issues.len(),
        status: "AWAITING_AI_REVIEW".into(), publication_permitted: false,
    };
    let token = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let stage = parent.join(format!(".kindlev2-stage-{}-{token}", std::process::id()));
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let operation = (|| -> Result<(), String> {
        write_new_json(&stage.join("audit.json"), &report)?;
        write_new_json(&stage.join("v2-case.json"), &case)?;
        write_new_json(&stage.join("decisions-pending.json"), &template)?;
        write_new_json(&stage.join("manifest.json"), &manifest)?;
        fs::write(stage.join("dashboard.html"), dashboard::render_html(&report))
            .map_err(|e| e.to_string())?;
        fs::rename(&stage, output).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if operation.is_err() { let _ = fs::remove_dir_all(&stage); }
    operation?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Chapter;
    #[test]
    fn creates_complete_intake_without_publishing_book() {
        let book = Book { title: "样本".into(), author: "作者".into(), source_id: "s".into(),
            source_url: "".into(), chapters: vec![Chapter { title: "第一章".into(),
                body: "自己的示例原文".repeat(30), source_id: "s".into(),
                source_url: "https://example.org/1".into(), ocr: false, ocr_evidence: None }] };
        let dir = std::env::temp_dir().join(format!("kindle-v2-intake-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let manifest = prepare(&book, None, &dir).unwrap();
        assert!(!manifest.publication_permitted);
        assert!(dir.join("v2-case.json").exists());
        assert!(dir.join("dashboard.html").exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
