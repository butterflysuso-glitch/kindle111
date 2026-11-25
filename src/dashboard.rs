use crate::model::AuditReport;

fn escape_html(input: &str) -> String {
    input.replace('&', "&amp;").replace('<', "&lt;")
        .replace('>', "&gt;").replace('"', "&quot;")
}

pub fn render_html(report: &AuditReport) -> String {
    let title = escape_html(&report.title);
    format!("<!doctype html><html lang=\"zh-CN\"><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>KindleProject 离线看板</title><style>body{{font-family:serif;\
background:#fff;color:#111;margin:1.5em;max-width:38em}}\
p{{border-bottom:1px solid #aaa;padding:.7em 0}}</style>\
<h1>{title}</h1><p>审核状态：待最终核验</p><p>章节数：{}</p>\
<p>待核问题：{}</p><p>完本认证：未通过</p>\
<small>离线静态面板，不提供正文，不要求后台服务。</small></html>",
        report.chapter_count, report.issues.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AuditReport;
    #[test]
    fn escapes_book_titles() {
        let report = AuditReport { schema: "test".into(), title: "<script>".into(), author: "".into(),
            chapter_count: 1, issues: vec![], chapter_hashes: vec![], completion_verified: false,
            publication_permitted: false, review_status: "PENDING".into() };
        assert!(!render_html(&report).contains("<script>"));
    }
}
