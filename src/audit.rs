use std::collections::{HashMap, HashSet};
use sha2::{Digest, Sha256};
use crate::model::{AuditReport, Book, Catalog, Issue};

fn normalized(text: &str) -> String {
    text.chars().filter(|ch| ch.is_alphanumeric())
        .flat_map(|ch| ch.to_lowercase()).collect()
}

fn url_valid(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && url.split("//").nth(1).is_some_and(|host| !host.is_empty() && !host.starts_with('@'))
}

pub fn sha256_hex(input: &[u8]) -> String {
    format!("{:x}", Sha256::digest(input))
}

pub fn audit(book: &Book, catalog: Option<&Catalog>) -> AuditReport {
    let mut issues = Vec::new();
    let mut hashes = Vec::new();
    let mut bodies = HashMap::new();
    let mut urls = HashSet::new();
    if book.title.trim().is_empty() || book.author.trim().is_empty() || book.source_id.is_empty() {
        issues.push(Issue { kind: "IDENTITY_MISSING".into(), chapter: None,
                            detail: "缺少书名、作者或来源身份".into() });
    }
    if book.chapters.is_empty() || book.chapters.len() > 10_000 {
        issues.push(Issue { kind: "CHAPTER_COUNT_INVALID".into(), chapter: None,
                            detail: "章节数量必须在 1–10000 之间".into() });
    }
    for (i, chapter) in book.chapters.iter().enumerate() {
        let index = i + 1;
        let compact: String = chapter.body.chars().filter(|c| !c.is_whitespace()).collect();
        let hash = sha256_hex(compact.as_bytes());
        hashes.push(hash.clone());
        if chapter.source_id != book.source_id {
            issues.push(Issue { kind: "MIXED_SOURCE".into(), chapter: Some(index),
                                detail: "章节来源与所选来源不同".into() });
        }
        if !url_valid(&chapter.source_url) {
            issues.push(Issue { kind: "SOURCE_URL_INVALID".into(), chapter: Some(index),
                                detail: "章节缺少可追踪的 URL".into() });
        }
        if !urls.insert(&chapter.source_url) {
            issues.push(Issue { kind: "DUPLICATE_URL".into(), chapter: Some(index),
                                detail: "章节 URL 重复".into() });
        }
        if compact.chars().count() < 100 {
            issues.push(Issue { kind: "SHORT_CHAPTER".into(), chapter: Some(index),
                                detail: "正文少于 100 字，需要人工核对".into() });
        }
        if let Some(previous) = bodies.insert(hash, index) {
            issues.push(Issue { kind: "DUPLICATE_BODY".into(), chapter: Some(index),
                                detail: format!("正文与第 {previous} 章一致") });
        }
        if chapter.ocr && chapter.ocr_evidence.as_ref().map_or(true, |record| {
            record.image_sha256.len() != 64 || record.review_status != "APPROVED"
        }) {
            issues.push(Issue { kind: "OCR_REVIEW_PENDING".into(), chapter: Some(index),
                                detail: "OCR 需要原图哈希及人工复核".into() });
        }
    }
    match catalog {
        None => issues.push(Issue { kind: "CATALOG_UNAVAILABLE".into(), chapter: None,
                                   detail: "缺少独立权威目录，不能认证完本".into() }),
        Some(reference) => {
            if normalized(&reference.title) != normalized(&book.title)
                || normalized(&reference.author) != normalized(&book.author) {
                issues.push(Issue { kind: "CATALOG_IDENTITY_MISMATCH".into(), chapter: None,
                                    detail: "权威目录与作品身份不一致".into() });
            } else {
                let titles: HashSet<String> = book.chapters.iter()
                    .map(|chapter| normalized(&chapter.title)).collect();
                let diff = reference.chapter_titles.iter()
                    .filter(|title| !titles.contains(&normalized(title))).count();
                if diff > 0 {
                    issues.push(Issue { kind: "CATALOG_TITLE_DIFFERENCES".into(), chapter: None,
                                        detail: format!("存在 {diff} 项目录标题差异；未自动认定为缺章") });
                }
            }
        }
    }
    AuditReport { schema: "kindleproject.v2.audit/2".into(), title: book.title.clone(),
                  author: book.author.clone(), chapter_count: book.chapters.len(),
                  issues, chapter_hashes: hashes, completion_verified: false,
                  publication_permitted: false, review_status: "PENDING_AI_REVIEW".into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Book, Chapter};

    fn sample() -> Book {
        Book { title: "例书".into(), author: "作者".into(), source_id: "user-source".into(),
               source_url: String::new(), chapters: vec![Chapter {
                   title: "第一章".into(), body: "正文".repeat(60),
                   source_id: "user-source".into(), source_url: "https://example.com/1".into(),
                   ocr: false, ocr_evidence: None }] }
    }

    #[test]
    fn no_catalog_blocks_publication() {
        let result = audit(&sample(), None);
        assert!(!result.publication_permitted);
        assert!(result.issues.iter().any(|issue| issue.kind == "CATALOG_UNAVAILABLE"));
    }

    #[test]
    fn mixed_source_is_visible() {
        let mut book = sample();
        book.chapters[0].source_id = "another".into();
        assert!(audit(&book, None).issues.iter().any(|issue| issue.kind == "MIXED_SOURCE"));
    }
}
