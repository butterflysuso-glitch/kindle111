# KindleProject V2 · Rust Baseline

**版本：2.0.0-alpha.2（迁移工作版）。这是可交接的 Rust 源码工程，不是已完成所有历史功能的正式成品。**

## 固定架构

- Rust 为唯一新业务代码语言。核心 CLI、章节审计、质量门禁、看板在本工程中直接实现。
- 用户可独立更新的书源和缓存都是外部运行时数据；**不纳入代码包，不固定来源数量，不在编译时嵌入**。
- AI 修书、真实性审核、缺章救援必须保留原 V1.8 的行为标准；Rust 迁移尚未完成，绝不伪称通过历史回归。
- 下载程序可以在独立目录运行；Rust 通过明确的数据契约读取其输出，不需要用户登录或内置站点兜底。
- EPUB/AZW3 与 Kindle 原生阅读是目标；当前没有实现正式发布功能，`gate` 始终拒绝最终发布。
- Kindle 离线看板无后台、无计划任务、无系统修改；本版只生成静态 HTML。

## 源码地图

| 文件 | 当前功能 |
| --- | --- |
| `src/model.rs` | 稳定的章节、书籍、目录、审计 JSON 数据结构 |
| `src/audit.rs` | 来源一致性、短章、重复、OCR 证据、目录差异、哈希审计 |
| `src/quality.rs` | 合并结构审计、审核案件、边界核验与来源证据的统一最终门禁 |
| `src/dashboard.rs` | 无网络的静态 HTML 看板 |
| `src/review.rs` | Rust 原生审核案件、完整正文和来源哈希绑定、相邻章节判定 |
| `src/repair.rs` | 缺章证据化暂存、权威目录定位和原文保护 |
| `src/epub.rs` | Rust 原生 EPUB3 待核预览和文件结构初检 |
| `src/incremental.rs` | 按整章哈希、来源哈希与相邻关系安全复用历史审核 |
| `src/convert.rs` | 调用 Calibre 转换待核 AZW3，180 秒超时、哈希比对、防止误覆盖 |
| `src/pipeline.rs` | 一次生成审计、审核案件、决策模板与静态看板，整目录原子交付 |
| `tests/integration.rs` | Rust CLI 端到端集成测试 |
| `src/main.rs` | 统一 CLI 入口 |
| `examples/` | 不含任何书源或版权正文的极简数据样例 |
| `docs/` | 迁移边界、交接清单与工程验收标准 |

## Windows 首次构建

1. 安装官方 Rust stable 工具链，确认 `cargo --version` 和 `rustc --version`。
2. 解压本包，打开项目根目录。
3. 执行 `cargo fmt --all -- --check`、`cargo test --locked`（首次没有 `Cargo.lock` 时先执行 `cargo generate-lockfile`）、`cargo build --release`。
4. 成功后运行 `target\release\kindleproject-v2.exe` 查看命令说明。

也可运行 `BUILD_WINDOWS.ps1`，它不会创建计划任务、启动项、服务或防火墙规则。

```powershell
target\release\kindleproject-v2.exe audit --book examples\book.json --catalog examples\catalog.json --out runs\audit.json
target\release\kindleproject-v2.exe dashboard --audit runs\audit.json --out runs\dashboard.html
target\release\kindleproject-v2.exe case --book examples\book.json --out runs\case.json --template runs\pending.json
target\release\kindleproject-v2.exe review --book examples\book.json --case runs\case.json --decisions runs\pending.json --out runs\review.json
target\release\kindleproject-v2.exe intake --book examples\book.json --catalog examples\catalog.json --out runs\sample-intake
target\release\kindleproject-v2.exe epub-preview --book examples\book.json --case runs\sample-intake\v2-case.json --decisions runs\sample-intake\decisions-pending.json --out runs\sample-preview.epub
target\release\kindleproject-v2.exe inspect-epub --file runs\sample-preview.epub --out runs\sample-structure.json
```

运行目录 `runs/`、动态书源目录 `data/`、凭证和各类生成文件均不参与源码版本管理。所有输出以新文件创建，禁止无提示覆盖。

## 对接下载程序

只接受标准化 JSON：`title`、`author`、`source_id`、`chapters`；每章含 `title`、`body`、`source_id`、`source_url`、可选 `ocr` 和 `ocr_evidence`。不执行书源中的 JavaScript、不主动联网搜索，也不修改外部书源。实际下载器输出格式与该契约不同的话，需要后续实现 Rust 导入适配器，不能假定已经接好。

## 验收状态（准确口径）

| 项目 | 状态 |
| --- | --- |
| Rust CLI、审计、看板、案件、补章、增量审核、EPUB3 预览、AZW3 待核转换及端到端测试源码 | 已编写，等待 Windows `cargo test` 实测 |
| Rust AI 审核/修书与旧 V1.8 等价 | **未完成** |
| 旧 V1.8 224 项回归用例迁移 | **未完成** |
| 自动换源下载结果 Rust 适配 | **未完成** |
| Rust EPUB 预览生成与结构初检 | 已编写，待编译与 EPUBCheck 实测 |
| Calibre AZW3 待核转换入口 | 已编写，等待 Windows 实机转换与哈希验收 |
| 独立真实性验证与最终成书认证 | **未完成；发布门禁保持关闭** |
| Kindle PNG/eips 离线部署实机验收 | **未完成** |
| Windows 清理及 Kindle 设备变更 | **未执行** |

**本包是可继续开发的 Rust V2 源码基线，不是全部功能已完成的正式版。**
