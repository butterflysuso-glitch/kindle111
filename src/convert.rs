"""Calibre process runner for review-only AZW3 with no shell expansion."""
use std::{fs::{self, File}, io::Read, path::Path, process::{Command, Stdio},
          thread, time::{Duration, Instant}};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionReport {
    pub mode: String,
    pub source_epub_sha256: String,
    pub azw3_sha256: String,
    pub size_bytes: u64,
    pub publication_permitted: bool,
}

fn file_hash(path: &Path) -> Result<String, String> {
    let mut input = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = [0u8; 65536];
    let mut digest = Sha256::new();
    loop {
        let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub fn convert_preview(epub: &Path, output: &Path, calibre_exe: &Path,
                       expected_sha256: &str) -> Result<ConversionReport, String> {
    if output.exists() { return Err("AZW3 输出已存在，拒绝覆盖".into()); }
    if output.extension().and_then(|e| e.to_str()).map(|e| !e.eq_ignore_ascii_case("azw3")) != Some(false)
        || !output.file_stem().and_then(|x| x.to_str()).unwrap_or("").to_lowercase().contains("preview") {
        return Err("AZW3 只能输出为带 preview 名称的待核文件".into());
    }
    if expected_sha256.len() != 64 || file_hash(epub)? != expected_sha256 {
        return Err("待核 EPUB 哈希不匹配，拒绝转换".into());
    }
    if !calibre_exe.is_file() { return Err("找不到指定的 Calibre ebook-convert 可执行文件".into()); }
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Direct executable and argument vector: never pass paths through cmd.exe.
    let mut child = Command::new(calibre_exe).arg(epub).arg(output)
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
    let started = Instant::now();
    let exit = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if started.elapsed() > Duration::from_secs(180) {
            let _ = child.kill();
            let _ = child.wait();
            if output.exists() { let _ = fs::remove_file(output); }
            return Err("Calibre 转换超过 180 秒，已停止该次启动的进程".into());
        }
        thread::sleep(Duration::from_millis(100));
    };
    if !exit.success() {
        if output.exists() { let _ = fs::remove_file(output); }
        return Err(format!("Calibre 转换失败，退出码 {:?}", exit.code()));
    }
    let metadata = fs::metadata(output).map_err(|e| e.to_string())?;
    if metadata.len() < 100 {
        let _ = fs::remove_file(output);
        return Err("输出 AZW3 文件异常过小".into());
    }
    Ok(ConversionReport { mode: "REVIEW_PREVIEW_ONLY".into(),
        source_epub_sha256: expected_sha256.into(), azw3_sha256: file_hash(output)?,
        size_bytes: metadata.len(), publication_permitted: false })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_filename_without_preview_before_running_program() {
        assert!(convert_preview(Path::new("missing.epub"), Path::new("final.azw3"),
                                Path::new("missing.exe"), "0").is_err());
    }
}
