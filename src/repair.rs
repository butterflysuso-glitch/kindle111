use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::{audit::sha256_hex, model::{Book, Catalog, Chapter, OcrEvidence}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replacement {
    pub title: String,
    pub body: String,
    pub source_url: String,
    pub acquisition: String,
    pub evidence_note: String,
    #[serde(default)] pub image_sha256: Option<String>,
    #[serde(default)] pub ocr_review: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairRecord {
    pub title: String,
    pub body_sha256: String,
    pub source_url: String,
    pub evidence_note: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub book: Book,
    pub inserted: Vec<RepairRecord>,
    pub publication_permitted: bool,
}

fn key(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

pub fn stage(book: &Book, catalog: &Catalog, fixes: &[Replacement]) -> Result<Candidate, String> {
    if key(&book.title) != key(&catalog.title) || key(&book.author) != key(&catalog.author) {
        return Err("权威目录身份不一致".into());
    }
    if fixes.is_empty() || catalog.chapter_titles.is_empty() {
        return Err("没有目录或补章记录".into());
    }
    let titles = catalog.chapter_titles.iter().map(|t| key(t)).collect::<Vec<_>>();
    if titles.iter().any(String::is_empty) || titles.iter().collect::<HashSet<_>>().len() != titles.len() {
        return Err("权威目录存在空标题或同名章节".into());
    }
    let mut rows = HashMap::new();
    for chapter in &book.chapters {
        let id = key(&chapter.title);
        if chapter.source_id != book.source_id || !titles.contains(&id) || rows.insert(id, chapter.clone()).is_some() {
            return Err("原书章节有歧义或混源".into());
        }
    }
    let mut evidence = Vec::new();
    for fix in fixes {
        let id = key(&fix.title);
        if !titles.contains(&id) || rows.contains_key(&id) {
            return Err("补章不存在于目录缺口，或将覆盖原文".into());
        }
        if fix.body.trim().chars().count() < 100 || fix.evidence_note.trim().chars().count() < 8
            || !(fix.source_url.starts_with("https://") || fix.source_url.starts_with("http://")) {
            return Err("补章正文或证据不充分".into());
        }
        if !["user_supplied", "authorized_web", "authorized_ocr"].contains(&fix.acquisition.as_str()) {
            return Err("补章获取方式无效".into());
        }
        let is_ocr = fix.acquisition == "authorized_ocr";
        let ocr_evidence = if is_ocr {
            let hash = fix.image_sha256.as_ref().ok_or("缺少 OCR 原图哈希")?;
            if hash.len() != 64 || fix.ocr_review.as_deref() != Some("APPROVED") {
                return Err("OCR 尚未完成证据复核".into());
            }
            Some(OcrEvidence { image_sha256: hash.clone(), review_status: "APPROVED".into() })
        } else { None };
        let title = catalog.chapter_titles[titles.iter().position(|x| x == &id).unwrap()].clone();
        rows.insert(id, Chapter { title: title.clone(), body: fix.body.clone(),
                                 source_id: book.source_id.clone(), source_url: fix.source_url.clone(),
                                 ocr: is_ocr, ocr_evidence });
        evidence.push(RepairRecord { title, body_sha256: sha256_hex(fix.body.as_bytes()),
                                     source_url: fix.source_url.clone(), evidence_note: fix.evidence_note.clone(),
                                     status: "PENDING_AI_REVIEW".into() });
    }
    let mut staged = book.clone();
    staged.chapters = titles.iter().filter_map(|k| rows.remove(k)).collect();
    Ok(Candidate { book: staged, inserted: evidence, publication_permitted: false })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> (Book, Catalog, Replacement) {
        let book = Book { title: "测试".into(), author: "作者".into(), source_id: "s".into(),
            source_url: "".into(), chapters: vec![Chapter { title: "第一章".into(), body: "甲".repeat(120),
                source_id: "s".into(), source_url: "https://example.org/1".into(),
                ocr: false, ocr_evidence: None }] };
        let catalog = Catalog { title: "测试".into(), author: "作者".into(),
                                chapter_titles: vec!["第一章".into(), "第二章".into()] };
        let fix = Replacement { title: "第二章".into(), body: "乙".repeat(120),
            source_url: "https://example.org/2".into(), acquisition: "user_supplied".into(),
            evidence_note: "用户明确提供的原文证据待核实".into(),
            image_sha256: None, ocr_review: None };
        (book, catalog, fix)
    }
    #[test]
    fn inserts_only_missing_chapter() {
        let (book, catalog, fix) = sample();
        let result = stage(&book, &catalog, &[fix.clone()]).unwrap();
        assert_eq!(result.book.chapters.len(), 2);
        assert_eq!(book.chapters.len(), 1);
        assert!(!result.publication_permitted);
        assert!(stage(&book, &catalog, &[Replacement { title: "第一章".into(), ..fix }]).is_err());
    }
    #[test]
    fn unreviewed_ocr_fails() {
        let (book, catalog, mut fix) = sample();
        fix.acquisition = "authorized_ocr".into();
        assert!(stage(&book, &catalog, &[fix]).is_err());
    }
}
