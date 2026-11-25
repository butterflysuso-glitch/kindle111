"""Consolidated gate for Rust V2 audit and chapter review reports."""
use serde::{Deserialize, Serialize};
use crate::{model::AuditReport, review::{Case, Verdict}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub case_id: String,
    pub catalog_url: String,
    pub reviewer_note: String,
    #[serde(default)]
    pub source_evidence_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GateResult {
    pub status: String,
    pub reasons: Vec<String>,
    pub publication_permitted: bool,
    pub independent_authenticity_verified: bool,
    pub human_signoff_required: bool,
}

pub fn evaluate(audit: &AuditReport, case: &Case, review: &Verdict,
                evidence: Option<&EvidenceRecord>) -> GateResult {
    let mut reasons = Vec::new();
    if !audit.issues.is_empty() {
        reasons.push(format!("结构审计仍有 {} 项问题", audit.issues.len()));
    }
    if case.title != audit.title || case.author != audit.author
        || case.entries.len() != audit.chapter_count || case.case_id != review.case_id {
        reasons.push("审计、审核案件或审核结论身份不一致".into());
    }
    let expected = audit.chapter_count.saturating_sub(1);
    if review.reviewed_pairs != expected || review.pending_pairs > 0 || review.disputed_pairs > 0 {
        reasons.push(format!("相邻章节审核未完成或存在疑义：应核对 {expected} 对"));
    }
    match evidence {
        None => reasons.push("缺少独立目录和来源真实性核查证据".into()),
        Some(record) => {
            if record.case_id != case.case_id
                || !(record.catalog_url.starts_with("https://") || record.catalog_url.starts_with("http://"))
                || record.reviewer_note.trim().chars().count() < 12
                || record.source_evidence_urls.is_empty() {
                reasons.push("来源/目录证据不足或绑定了错误案件".into());
            }
        }
    }
    reasons.push("独立真实性与完本认证尚未迁移，禁止自动发布".into());
    GateResult { status: "PENDING_FINAL_CERTIFICATION".into(), reasons,
                 publication_permitted: false, independent_authenticity_verified: false,
                 human_signoff_required: true }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn never_claims_unimplemented_certification() {
        let audit = AuditReport { schema: "test".into(), title: "测试".into(), author: "作者".into(),
            chapter_count: 1, issues: vec![], chapter_hashes: vec!["hash".into()],
            completion_verified: false, publication_permitted: false, review_status: "PENDING".into() };
        let case = Case { case_id: "id".into(), title: "测试".into(), author: "作者".into(),
            source_id: "source".into(), entries: vec![crate::review::Entry {
                id: "C00001".into(), title: "一".into(), body_hash: "h".into(), source_hash: "s".into() }] };
        let verdict = Verdict { case_id: "id".into(), reviewed_pairs: 0,
            pending_pairs: 0, disputed_pairs: 0, publication_permitted: false };
        assert!(!evaluate(&audit, &case, &verdict, None).publication_permitted);
    }
}
