$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw '未发现 cargo；请先安装官方 Rust 工具链。'
}
Write-Host '== KindleProject V2 Rust baseline =='
cargo --version
rustc --version
cargo generate-lockfile
cargo fmt --all
cargo fmt --all -- --check
cargo test --locked
cargo build --locked --release
Write-Host '编译完成：target\release\kindleproject-v2.exe'
Write-Host '没有创建计划任务、系统服务或启动项。'
