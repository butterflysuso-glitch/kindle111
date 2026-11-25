use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use crate::{audit::sha256_hex, model::Book};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub body_hash: String,
    pub source_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub case_id: String,
    pub title: String,
    pub author: String,
    pub source_id: String,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairReview {
    pub before: String,
    pub after: String,
    pub verdict: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decisions {
    pub case_id: String,
    pub order: Vec<String>,
    pub pairs: Vec<PairReview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub case_id: String,
    pub reviewed_pairs: usize,
    pub pending_pairs: usize,
    pub disputed_pairs: usize,
    pub publication_permitted: bool,
}

pub fn make_case(book: &Book) -> Result<Case, String> {
    if book.chapters.is_empty() || book.chapters.len() > 10000 {
        return Err("章节数超出安全范围".into());
    }
    if book.chapters.iter().any(|c| c.source_id != book.source_id) {
        return Err("不能把跨源混章作为同一来源案件".into());
    }
    let entries = book.chapters.iter().enumerate().map(|(i, ch)| Entry {
        id: format!("C{:05}", i + 1), title: ch.title.clone(),
        body_hash: sha256_hex(ch.body.as_bytes()),
        source_hash: sha256_hex(ch.source_url.as_bytes()),
    }).collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&(&book.title, &book.author, &book.source_id, &entries))
        .map_err(|e| e.to_string())?;
    Ok(Case { case_id: sha256_hex(&bytes), title: book.title.clone(),
              author: book.author.clone(), source_id: book.source_id.clone(), entries })
}

pub fn template(case: &Case) -> Decisions {
    Decisions { case_id: case.case_id.clone(),
                order: case.entries.iter().map(|e| e.id.clone()).collect(),
                pairs: Vec::new() }
}

pub fn validate(book: &Book, case: &Case, decision: &Decisions) -> Result<Verdict, String> {
    let live = make_case(book)?;
    if live.case_id != case.case_id || live.entries != case.entries {
        return Err("正文、标题或来源与原审核案件不一致".into());
    }
    if decision.case_id != case.case_id { return Err("case_id 不一致".into()); }
    let original: HashSet<&str> = case.entries.iter().map(|e| e.id.as_str()).collect();
    let proposed: HashSet<&str> = decision.order.iter().map(String::as_str).collect();
    if proposed != original || proposed.len() != decision.order.len() {
        return Err("审核顺序必须保留全部原章节且恰好一次".into());
    }
    let adjacent: HashSet<(&str, &str)> = decision.order.windows(2)
        .map(|p| (p[0].as_str(), p[1].as_str())).collect();
    let mut checked = HashSet::new();
    let mut disputed = 0;
    for pair in &decision.pairs {
        let link = (pair.before.as_str(), pair.after.as_str());
        if !adjacent.contains(&link) || !checked.insert(link) {
            return Err("存在无效或重复的相邻章节审核".into());
        }
        if pair.reason.trim().chars().count() < 8 {
            return Err("缺少具体审核理由".into());
        }
        match pair.verdict.as_str() {
            "coherent" => {},
            "suspect" | "uncertain" => disputed += 1,
            _ => return Err("未知审核结论".into()),
        }
    }
    Ok(Verdict { case_id: case.case_id.clone(), reviewed_pairs: checked.len(),
                 pending_pairs: adjacent.len() - checked.len(), disputed_pairs: disputed,
                 publication_permitted: false })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Chapter;
    fn book() -> Book {
        Book { title: "测试".into(), author: "作者".into(), source_id: "u".into(), source_url: "".into(),
            chapters: vec![Chapter { title: "第一章".into(), body: "原文".repeat(80),
                source_id: "u".into(), source_url: "https://example.org/1".into(),
                ocr: false, ocr_evidence: None }] }
    }
    #[test]
    fn changed_body_breaks_case() {
        let base = book(); let case = make_case(&base).unwrap();
        let mut modified = base.clone(); modified.chapters[0].body.push('错');
        assert!(validate(&modified, &case, &template(&case)).is_err());
    }
    #[test]
    fn valid_case_is_never_auto_published() {
        let b = book(); let c = make_case(&b).unwrap();
        assert!(!validate(&b, &c, &template(&c)).unwrap().publication_permitted);
    }
}
