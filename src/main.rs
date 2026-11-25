use std::{env, path::PathBuf, process};
use kindleproject_v2::{audit, convert, dashboard, epub, incremental, pipeline, model::{AuditReport, Book, Catalog},
                       quality::{self, EvidenceRecord}, review::{self, Case, Decisions, Verdict},
                       repair::{self, Replacement},
                       read_json, write_new_json};

fn argument(args: &[String], key: &str) -> Result<PathBuf, String> {
    let position = args.iter().position(|value| value == key)
        .ok_or_else(|| format!("缺少参数 {key}"))?;
    args.get(position + 1).map(PathBuf::from)
        .ok_or_else(|| format!("参数 {key} 缺少路径"))
}

fn text_argument(args: &[String], key: &str) -> Result<String, String> {
    let position = args.iter().position(|value| value == key)
        .ok_or_else(|| format!("缺少参数 {key}"))?;
    args.get(position + 1).cloned().ok_or_else(|| format!("参数 {key} 缺少值"))
}

fn usage() {
    eprintln!("KindleProject V2 Rust alpha\n\
  kindleproject-v2 audit --book book.json --out audit.json [--catalog catalog.json]\n\
  kindleproject-v2 dashboard --audit audit.json --out status.html\n\
  kindleproject-v2 gate --audit audit.json --case case.json --review result.json --out gate.json [--evidence evidence.json]\n\
  kindleproject-v2 case --book book.json --out case.json --template pending.json\n\
  kindleproject-v2 review --book book.json --case case.json --decisions filled.json --out result.json\n\
  kindleproject-v2 repair --book book.json --catalog catalog.json --replacements fixes.json --out staged.json\n\
  kindleproject-v2 epub-preview --book book.json --case case.json --decisions filled.json --out book-preview.epub\n\
  kindleproject-v2 inspect-epub --file book-preview.epub --out structure.json\n\
  kindleproject-v2 reuse --old-case old.json --old-decisions old-decisions.json --new-case new.json --new-template pending.json --out reused.json\n\
  kindleproject-v2 convert-preview --epub review.epub --calibre C:\\Calibre\\ebook-convert.exe --sha256 HASH --out book-preview.azw3 --report convert.json\n\
  kindleproject-v2 intake --book book.json [--catalog catalog.json] --out runs/new-task\n\
所有输出默认不覆盖；不访问网络、不执行书源脚本、不写入 Kindle。");
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        usage();
        return Err("未提供命令".into());
    };
    match command {
        "audit" => {
            let book: Book = read_json(&argument(&args, "--book")?)?;
            let catalog = if args.iter().any(|value| value == "--catalog") {
                Some(read_json::<Catalog>(&argument(&args, "--catalog")?)?)
            } else { None };
            let report = audit::audit(&book, catalog.as_ref());
            write_new_json(&argument(&args, "--out")?, &report)?;
            println!("审核案件待完成：章节 {}，问题 {}", report.chapter_count, report.issues.len());
        }
        "intake" => {
            let book: Book = read_json(&argument(&args, "--book")?)?;
            let catalog = if args.iter().any(|value| value == "--catalog") {
                Some(read_json::<Catalog>(&argument(&args, "--catalog")?)?)
            } else { None };
            let result = pipeline::prepare(&book, catalog.as_ref(), &argument(&args, "--out")?)?;
            println!("V2 案件 {} 已建立；问题 {}，等待真实 AI 审核",
                     result.case_id, result.issue_count);
        }
        "dashboard" => {
            let report: AuditReport = read_json(&argument(&args, "--audit")?)?;
            let destination = argument(&args, "--out")?;
            if let Some(parent) = destination.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let html = dashboard::render_html(&report);
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new().create_new(true).write(true)
                .open(&destination).map_err(|e| e.to_string())?;
            file.write_all(html.as_bytes()).map_err(|e| e.to_string())?;
            println!("静态看板已创建；无需后台服务");
        }
        "gate" => {
            let audit: AuditReport = read_json(&argument(&args, "--audit")?)?;
            let case: Case = read_json(&argument(&args, "--case")?)?;
            let verdict: Verdict = read_json(&argument(&args, "--review")?)?;
            let evidence: Option<EvidenceRecord> = if args.iter().any(|s| s == "--evidence") {
                Some(read_json(&argument(&args, "--evidence")?)?)
            } else { None };
            let report = quality::evaluate(&audit, &case, &verdict, evidence.as_ref());
            write_new_json(&argument(&args, "--out")?, &report)?;
            println!("审核门禁状态：{}；最终发布被阻止", report.status);
        }
        "case" => {
            let book: Book = read_json(&argument(&args, "--book")?)?;
            let case = review::make_case(&book)?;
            let template_path = argument(&args, "--template")?;
            let case_path = argument(&args, "--out")?;
            if case_path.exists() || template_path.exists() {
                return Err("案件或审核模板已存在".into());
            }
            write_new_json(&case_path, &case)?;
            write_new_json(&template_path, &review::template(&case))?;
            println!("审核案件生成成功；仍需真实来源核查");
        }
        "review" => {
            let book: Book = read_json(&argument(&args, "--book")?)?;
            let case: Case = read_json(&argument(&args, "--case")?)?;
            let decision: Decisions = read_json(&argument(&args, "--decisions")?)?;
            let result = review::validate(&book, &case, &decision)?;
            write_new_json(&argument(&args, "--out")?, &result)?;
            println!("已核对 {} 对；剩余 {} 对", result.reviewed_pairs, result.pending_pairs);
        }
        "repair" => {
            let book: Book = read_json(&argument(&args, "--book")?)?;
            let catalog: Catalog = read_json(&argument(&args, "--catalog")?)?;
            let fixes: Vec<Replacement> = read_json(&argument(&args, "--replacements")?)?;
            let candidate = repair::stage(&book, &catalog, &fixes)?;
            write_new_json(&argument(&args, "--out")?, &candidate)?;
            println!("暂存 {} 个补章候选；必须重新审查", candidate.inserted.len());
        }
        "epub-preview" => {
            let book: Book = read_json(&argument(&args, "--book")?)?;
            let case: Case = read_json(&argument(&args, "--case")?)?;
            let decisions: Decisions = read_json(&argument(&args, "--decisions")?)?;
            epub::create_preview(&book, &case, &decisions, &argument(&args, "--out")?)?;
            println!("Rust EPUB 待核预览已生成；不可作为正式完本发布");
        }
        "inspect-epub" => {
            let chapters = epub::inspect(&argument(&args, "--file")?)?;
            write_new_json(&argument(&args, "--out")?,
                &serde_json::json!({"chapter_count": chapters, "structure_checked": true,
                                   "publication_permitted": false, "completion_verified": false}))?;
            println!("EPUB 结构初检：{chapters} 章；不代表完本认证");
        }
        "reuse" => {
            let old_case: Case = read_json(&argument(&args, "--old-case")?)?;
            let old_decisions: Decisions = read_json(&argument(&args, "--old-decisions")?)?;
            let new_case: Case = read_json(&argument(&args, "--new-case")?)?;
            let new_template: Decisions = read_json(&argument(&args, "--new-template")?)?;
            let reused = incremental::reuse(&old_case, &old_decisions, &new_case, &new_template)?;
            write_new_json(&argument(&args, "--out")?, &reused)?;
            println!("可复用 {} 对，剩余 {} 对待审核", reused.reused_pairs, reused.pending_pairs);
        }
        "convert-preview" => {
            let target = argument(&args, "--out")?;
            let report_path = argument(&args, "--report")?;
            if report_path.exists() { return Err("转换报告已存在".into()); }
            let report = convert::convert_preview(&argument(&args, "--epub")?, &target,
                         &argument(&args, "--calibre")?, &text_argument(&args, "--sha256")?)?;
            write_new_json(&report_path, &report)?;
            println!("已转换待核 AZW3；仍禁止正式发布");
        }
        _ => { usage(); return Err(format!("未知命令 {command}")); }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("错误：{error}");
        process::exit(2);
    }
}
