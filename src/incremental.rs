"""Reuse only proven, unchanged, still-adjacent Rust review pairs."""
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::review::{Case, Decisions, PairReview};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReuseResult {
    pub decisions: Decisions,
    pub reused_pairs: usize,
    pub pending_pairs: usize,
    pub prior_case_id: String,
    pub current_case_id: String,
    pub publication_permitted: bool,
}

fn identity(title: &str, body_hash: &str, source_hash: &str) -> String {
    // Length-prefix fields to avoid ambiguous concatenation.
    format!("{}:{title}{}:{body_hash}{}:{source_hash}",
            title.len(), body_hash.len(), source_hash.len())
}

pub fn reuse(old_case: &Case, old_decisions: &Decisions,
             new_case: &Case, new_template: &Decisions) -> Result<ReuseResult, String> {
    if old_case.case_id != old_decisions.case_id || new_case.case_id != new_template.case_id {
        return Err("案件和决策不匹配".into());
    }
    if old_case.title != new_case.title || old_case.author != new_case.author
        || old_case.source_id != new_case.source_id {
        return Err("作品或来源身份变化，不能复用审核".into());
    }
    let old_ids: HashSet<&str> = old_case.entries.iter().map(|e| e.id.as_str()).collect();
    if old_decisions.order.len() != old_ids.len()
        || old_decisions.order.iter().map(String::as_str).collect::<HashSet<_>>() != old_ids {
        return Err("旧决策缺失、重复或增加了章节".into());
    }
    let old_adjacent: HashSet<(&str, &str)> = old_decisions.order.windows(2)
        .map(|w| (w[0].as_str(), w[1].as_str())).collect();
    let mut seen = HashSet::new();
    for pair in &old_decisions.pairs {
        let edge = (pair.before.as_str(), pair.after.as_str());
        if !old_adjacent.contains(&edge) || !seen.insert(edge)
            || pair.reason.trim().chars().count() < 8
            || !["coherent", "suspect", "uncertain"].contains(&pair.verdict.as_str()) {
            return Err("旧审核数据无效，不能复用".into());
        }
    }
    let mut old_keys = HashMap::new();
    for entry in &old_case.entries {
        let key = identity(&entry.title, &entry.body_hash, &entry.source_hash);
        if old_keys.insert(entry.id.as_str(), key).is_some() {
            return Err("旧案件章节 ID 重复".into());
        }
    }
    let mut new_lookup = HashMap::new();
    for entry in &new_case.entries {
        let key = identity(&entry.title, &entry.body_hash, &entry.source_hash);
        if new_lookup.insert(key, entry.id.as_str()).is_some() {
            return Err("新案件存在重复的章节身份".into());
        }
    }
    if old_keys.values().collect::<HashSet<_>>().len() != old_keys.len() {
        return Err("旧案件存在重复的章节身份".into());
    }
    let mut eligible = HashMap::new();
    for previous in &old_decisions.pairs {
        if previous.verdict != "coherent" { continue; }
        let Some(before) = old_keys.get(previous.before.as_str()).and_then(|key| new_lookup.get(key)) else { continue; };
        let Some(after) = old_keys.get(previous.after.as_str()).and_then(|key| new_lookup.get(key)) else { continue; };
        eligible.insert(((*before).to_owned(), (*after).to_owned()), previous);
    }
    let mut result = new_template.clone();
    result.pairs.clear();
    for adjacent in new_template.order.windows(2) {
        if let Some(previous) = eligible.get(&(adjacent[0].clone(), adjacent[1].clone())) {
            result.pairs.push(PairReview {
                before: adjacent[0].clone(), after: adjacent[1].clone(), verdict: "coherent".into(),
                reason: format!("原案 {} 整章与来源不变，沿用已核证边界：{}",
                                &old_case.case_id[..old_case.case_id.len().min(12)], previous.reason),
            });
        }
    }
    let reused = result.pairs.len();
    let pending = new_template.order.len().saturating_sub(1).saturating_sub(reused);
    Ok(ReuseResult { decisions: result, reused_pairs: reused, pending_pairs: pending,
                     prior_case_id: old_case.case_id.clone(), current_case_id: new_case.case_id.clone(),
                     publication_permitted: false })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::{Book, Chapter}, review::{make_case, template}};
    fn book() -> Book {
        Book { title: "示例".into(), author: "作者".into(), source_id: "s".into(), source_url: "".into(),
            chapters: (1..=3).map(|i| Chapter { title: format!("第{i}章"),
                body: format!("测试正文{i}").repeat(50), source_id: "s".into(),
                source_url: format!("https://example.org/{i}"), ocr: false, ocr_evidence: None }).collect() }
    }
    #[test]
    fn insertion_only_invalidates_neighboring_edges() {
        let original = book(); let old = make_case(&original).unwrap();
        let mut decisions = template(&old);
        decisions.pairs = (1..=2).map(|i| PairReview { before: format!("C{i:05}"),
            after: format!("C{:05}", i+1), verdict: "coherent".into(),
            reason: "完整正文与相邻剧情均核对通过".into() }).collect();
        let mut updated = original.clone();
        updated.chapters.insert(1, Chapter { title: "插入章".into(), body: "测试插章".repeat(40),
            source_id: "s".into(), source_url: "https://example.org/new".into(),
            ocr: false, ocr_evidence: None });
        let next = make_case(&updated).unwrap();
        let result = reuse(&old, &decisions, &next, &template(&next)).unwrap();
        assert_eq!(result.reused_pairs, 1);
        assert_eq!(result.pending_pairs, 2);
        assert!(!result.publication_permitted);
    }
}
