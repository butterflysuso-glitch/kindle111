use std::{fs::{File, OpenOptions}, io::{Read, Write}, path::Path};
use zip::{ZipWriter, CompressionMethod, write::FileOptions};
use crate::{model::Book, review::{Case, Decisions, validate}};

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        .replace('"', "&quot;").replace('\'', "&apos;")
}

fn xhtml(title: &str, body: &str) -> String {
    let content = body.lines().filter(|line| !line.trim().is_empty())
        .map(|line| format!("<p>{}</p>", xml(line.trim()))).collect::<Vec<_>>().join("");
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"zh\"><head><title>{}</title></head><body><h1>{}</h1>{}</body></html>",
            xml(title), xml(title), content)
}

pub fn create_preview(book: &Book, case: &Case, decision: &Decisions, output: &Path) -> Result<(), String> {
    let verdict = validate(book, case, decision)?;
    if verdict.pending_pairs > 0 || verdict.disputed_pairs > 0 {
        return Err("审核未完成，禁止生成 EPUB".into());
    }
    if output.exists() { return Err("文件已存在，不允许覆盖".into()); }
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let writer = OpenOptions::new().write(true).create_new(true).open(output)
        .map_err(|e| e.to_string())?;
    let mut archive = ZipWriter::new(writer);
    let stored = FileOptions::default().compression_method(CompressionMethod::Stored);
    let compressed = FileOptions::default().compression_method(CompressionMethod::Deflated);
    let result = (|| -> Result<(), String> {
        archive.start_file("mimetype", stored).map_err(|e| e.to_string())?;
        archive.write_all(b"application/epub+zip").map_err(|e| e.to_string())?;
        archive.start_file("META-INF/container.xml", compressed).map_err(|e| e.to_string())?;
        archive.write_all(br#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#)
            .map_err(|e| e.to_string())?;
        let mut manifest = String::new();
        let mut spine = String::new();
        let mut navigation = String::new();
        for (i, id) in decision.order.iter().enumerate() {
            let original = case.entries.iter().position(|entry| &entry.id == id)
                .ok_or_else(|| "章节 ID 不匹配".to_string())?;
            let chapter = &book.chapters[original];
            let name = format!("chapter-{i:05}.xhtml");
            archive.start_file(format!("OEBPS/{name}"), compressed).map_err(|e| e.to_string())?;
            archive.write_all(xhtml(&chapter.title, &chapter.body).as_bytes()).map_err(|e| e.to_string())?;
            manifest.push_str(&format!("<item id=\"c{i}\" href=\"{name}\" media-type=\"application/xhtml+xml\"/>"));
            spine.push_str(&format!("<itemref idref=\"c{i}\"/>"));
            navigation.push_str(&format!("<li><a href=\"{name}\">{}</a></li>", xml(&chapter.title)));
        }
        archive.start_file("OEBPS/nav.xhtml", compressed).map_err(|e| e.to_string())?;
        let nav = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\"><head><title>目录</title></head><body><nav epub:type=\"toc\"><h1>目录</h1><ol>{navigation}</ol></nav></body></html>");
        archive.write_all(nav.as_bytes()).map_err(|e| e.to_string())?;
        archive.start_file("OEBPS/content.opf", compressed).map_err(|e| e.to_string())?;
        let opf = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"uid\"><metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:identifier id=\"uid\">urn:sha256:{}</dc:identifier><dc:title>{}</dc:title><dc:creator>{}</dc:creator><dc:language>zh</dc:language><meta property=\"dcterms:modified\">2026-11-25T00:00:00Z</meta></metadata><manifest>{manifest}<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/></manifest><spine>{spine}</spine></package>",
            case.case_id, xml(&format!("【待最终核验·预览】{}", book.title)), xml(&book.author));
        archive.write_all(opf.as_bytes()).map_err(|e| e.to_string())?;
        archive.finish().map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() { let _ = std::fs::remove_file(output); }
    result
}

pub fn inspect(path: &Path) -> Result<usize, String> {
    let input = File::open(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(input).map_err(|e| e.to_string())?;
    if zip.by_index(0).map_err(|e| e.to_string())?.name() != "mimetype" {
        return Err("EPUB mimetype 未放在第一项".into());
    }
    let mut kind = String::new();
    zip.by_name("mimetype").map_err(|e| e.to_string())?
        .read_to_string(&mut kind).map_err(|e| e.to_string())?;
    if kind != "application/epub+zip" { return Err("文件不是有效 EPUB".into()); }
    let mut opf = String::new();
    zip.by_name("OEBPS/content.opf").map_err(|e| e.to_string())?
        .read_to_string(&mut opf).map_err(|e| e.to_string())?;
    let count = opf.matches("<itemref ").count();
    if count == 0 { return Err("正文目录为空".into()); }
    for i in 0..count {
        zip.by_name(&format!("OEBPS/chapter-{i:05}.xhtml"))
            .map_err(|_| format!("正文文件 {i} 缺失"))?;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::Chapter, review::{make_case, template}};
    #[test]
    fn single_chapter_preview_can_be_inspected() {
        let book = Book { title: "示例".into(), author: "作者".into(), source_id: "u".into(),
            source_url: "".into(), chapters: vec![Chapter {
                title: "第一章".into(), body: "测试正文".repeat(100), source_id: "u".into(),
                source_url: "https://example.org/1".into(), ocr: false, ocr_evidence: None }] };
        let case = make_case(&book).unwrap();
        let decisions = template(&case);
        let target = std::env::temp_dir().join(format!("kindle-v2-{}-{}.epub", std::process::id(), &case.case_id[..10]));
        let _ = std::fs::remove_file(&target);
        create_preview(&book, &case, &decisions, &target).unwrap();
        assert_eq!(inspect(&target).unwrap(), 1);
        std::fs::remove_file(&target).unwrap();
    }
}
