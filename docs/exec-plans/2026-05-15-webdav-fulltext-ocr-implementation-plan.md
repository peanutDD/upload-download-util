# WebDAV + 全文搜索 + OCR 实现计划

Date: 2026-05-15

## Goal

按“适度优化”完成 WebDAV、全文搜索、OCR 业务代码重构与实现收敛。外部 HTTP API、响应结构、认证方式、配置项和既有业务行为保持兼容；内部代码拆成清晰模块，降低 API handler、service、worker 混杂，补足关键协议注释、文档和回归护栏。

## Scope

包含：

- WebDAV API 模块化拆分和 service 边界收敛。
- 全文搜索 Tantivy 服务边界、snippet/source 逻辑和索引编排解耦。
- OCR facade、图片/PDF/runtime helper 拆分。
- 文档、约束、质量评分和测试门禁。

不包含：

- 不改前端 UI，除非后端类型变更要求同步。
- 不做数据库迁移。
- 不改变 Tantivy schema。
- 不引入新的 Rust 运行时依赖。
- 不提交 `backend/search-index/` 等运行产物。

## Assumptions

- WebDAV 外部入口保持 `/dav` 和已有 `/api/v1/...` 路径兼容。
- WebDAV Basic auth 继续使用 `username:api_token`，账号密码仍不可作为 WebDAV 凭证。
- 全文搜索继续由 `FULLTEXT_SEARCH_ENABLED`、`SEARCH_INDEX_PATH` 控制。
- OCR 继续由 `OCR_ENABLED`、`OCR_TESSERACT_BIN`、`OCR_PDFTOPPM_BIN`、`OCR_PDF_MAX_PAGES` 控制。
- OCR 缺依赖、禁用或不支持文件类型时必须降级为可观测状态，不能打断 worker。
- 当前工作区可能有既有脏改；实施时只编辑本计划涉及文件，不回滚他人改动。

## Risks

- WebDAV 协议兼容面广，Finder/rclone/macOS 行为尤其容易在 `Destination`、`Depth`、Range、LOCK/UNLOCK 上回归。
- WebDAV COPY/MOVE/DELETE 涉及文件、文件夹、缓存、全文索引入队等多个副作用，必须集中在 service 层。
- Tantivy writer 需要保持单 writer/共享 index 管理，避免 LockBusy。
- OCR PDF 渲染和 Tesseract 调用是外部命令边界，必须控制页数和失败降级。
- 集成测试共用 PostgreSQL 时，清理测试数据的用例需要串行执行，避免互删 fixture。

## Dependencies

- Existing backend stack: Axum, SQLx, Tantivy, quick-xml, metrics.
- Existing services: `FileService`, `FolderService`, storage backend, background task queue.
- Runtime OCR binaries: Tesseract and Poppler `pdftoppm`.
- Existing tests under `backend/tests/handler_webdav_tests.rs` and `backend/tests/fulltext_search_tests.rs`.

## Phase 1: WebDAV API 模块化拆分

目标：把 WebDAV 单文件 API 拆成固定模块，入口稳定、协议 helper 可单测。

Target files:

- `backend/src/api/webdav/mod.rs`: `/dav` router、method dispatch、OPTIONS、统一错误映射、请求指标。
- `backend/src/api/webdav/auth.rs`: Basic/Bearer token 认证适配、WebDAV root folder 校验。
- `backend/src/api/webdav/methods.rs`: MKCOL/PUT/GET/HEAD/DELETE/MOVE/COPY 的 HTTP header/body 读取和 response 构造。
- `backend/src/api/webdav/propfind.rs`: PROPFIND 请求解析、Depth 遍历、Multi-Status XML。
- `backend/src/api/webdav/lock.rs`: Class 2 LOCK/UNLOCK、active lock discovery、锁冲突查询。
- `backend/src/api/webdav/path.rs`: DAV path 清洗、Destination 兼容解析、href 编码。
- `backend/src/api/webdav/range.rs`: 单段 Range 解析。
- `backend/src/api/webdav/lock_headers.rs`: `Lock-Token` / `If` token 解析。
- `backend/src/api/webdav/xml_fragments.rs`: DAV XML fragment 构造和 XML 转义。

Implementation checklist:

- 保留 `create_router()` 和 `handle_root()` 对外入口。
- 纯函数先覆盖单测：路径清洗、Destination 解析、href 编码、Range 解析、Lock-Token/If 解析、XML 转义。
- 注释只放协议容易误读处：`Depth: infinity` 上限、`Destination` 反代/相对路径兼容、Class 2 lock 语义、macOS AppleDouble 忽略。
- `mod.rs` 不直接拼 WebDAV 业务流程。

Acceptance:

- `backend/src/api/webdav/mod.rs` 只做入口、dispatch、OPTIONS、统一错误映射、指标。
- WebDAV handler 回归测试保持现有行为。

## Phase 2: WebDAV 业务逻辑下沉到 Service

目标：新增并稳定 `services::webdav::WebDavService`，API 层只处理 HTTP，业务副作用集中在 service。

Target files:

- `backend/src/services/webdav.rs`
- `backend/src/services/mod.rs`
- `backend/src/repositories/folders.rs`

Domain types:

- `WebDavPrincipal`
- `DavResource`
- `WebDavError`
- `WebDavMoveOutcome`
- `WebDavPutInput`
- `WebDavReadFile`
- `WebDavChildren`

Service responsibilities:

- 资源解析：root folder、父目录、文件/文件夹查找。
- 创建集合：MKCOL 对应 folder create。
- PUT 文件：接收 API 层写好的临时文件，调用 file service 入库，触发 cache bump。
- GET/HEAD：返回文件元数据和 stream 决策。
- DELETE：删除文件或递归删除文件夹树。
- MOVE/COPY：处理 overwrite、目标冲突、文件夹递归复制、文件移动。
- cache bump 和 fulltext 入队。
- 使用按父目录和名称查 folder 的 repository 方法，避免加载整个 sibling 列表。

API responsibilities:

- 读取 header/body。
- 临时文件落盘和上传大小限制。
- Range header 解析和 response header 构造。
- 状态码映射。
- 指标标签。

Acceptance:

- API 层不直接做文件/文件夹业务写入。
- `WebDavError` 统一映射到 HTTP status。
- cache/fulltext side effects 不丢失。

## Phase 3: 全文搜索与 OCR 索引编排解耦

目标：保留对外模块和类型兼容，同时把搜索、索引编排、OCR runtime 分层。

Target files:

- `backend/src/services/fulltext_search.rs`
- `backend/src/services/fulltext_search/snippet.rs`
- `backend/src/services/fulltext_indexer.rs`
- `backend/src/services/task_queue.rs`
- `backend/src/services/ocr.rs`
- `backend/src/services/ocr/image.rs`
- `backend/src/services/ocr/pdf.rs`
- `backend/src/services/ocr/runtime.rs`

Implementation checklist:

- 保留 `SearchIndexService`、`SearchDocument`、`SearchHit` 调用兼容。
- `fulltext_search` 内部拆出 snippet/source 判定。
- 新增 `FulltextIndexer`，承接 `search_index_file` 的业务编排：
  - 读取文件。
  - 普通文本提取。
  - OCR 提取。
  - OCR status 指标和日志。
  - 构造 `SearchDocument`。
  - 写 Tantivy。
- `task_queue.rs` 只保留任务 dequeue/retry/mark succeeded/failed，不直接知道 OCR 和文档构造细节。
- `OcrExtractor::extract_with_options`、`OcrOptions`、`OcrOutcome`、`OcrStatus` 保持兼容。
- OCR 注释聚焦外部命令边界：
  - Tesseract 语言固定为 `eng`。
  - PDF OCR 受 `OCR_PDF_MAX_PAGES` 限制。
  - 缺依赖必须降级而非打断 worker。

Acceptance:

- 上传 worker 可索引文本和 OCR 文本。
- 删除 worker 可移除索引。
- OCR dependency missing、PDF page limit、OCR status API 均有回归覆盖。

## Phase 4: 文档、约束和质量收尾

目标：把新边界写入 docs，使后续 agent 和人类 reviewer 能快速定位责任边界。

Documents:

- `docs/references/fulltext-ocr-usage.md`: 更新模块边界、排查方式、配置说明。
- `docs/constraints/C-088-webdav-api-service-boundary.md`: WebDAV API 层不得直接承载业务副作用。
- `docs/constraints/C-089-shared-db-tests-must-be-serial.md`: 共享 DB 测试必须串行，避免清理竞态。
- `docs/quality-score.md`: 记录质量分、验证命令和残余风险。

Acceptance:

- docs 能说明 WebDAV、全文搜索、OCR 的当前实现边界。
- 新约束可防止 WebDAV API 层重新膨胀、共享 DB 测试继续 flake。

## Public API Compatibility

HTTP path compatibility:

- `/dav`
- `/api/files/search/fulltext`
- `/api/files/search/ocr/status`
- `/api/v1/...`

WebDAV method compatibility:

- `OPTIONS`
- `PROPFIND`
- `MKCOL`
- `PUT`
- `GET`
- `HEAD`
- `DELETE`
- `MOVE`
- `COPY`
- `LOCK`
- `UNLOCK`

Configuration compatibility:

- `FULLTEXT_SEARCH_ENABLED`
- `SEARCH_INDEX_PATH`
- `OCR_ENABLED`
- `OCR_TESSERACT_BIN`
- `OCR_PDFTOPPM_BIN`
- `OCR_PDF_MAX_PAGES`

## Test Plan

Targeted WebDAV:

```bash
cd backend && cargo test --test handler_webdav_tests webdav_propfind
cd backend && cargo test --test handler_webdav_tests webdav_move_accepts_reverse_proxy_destination_prefix
cd backend && cargo test --test handler_webdav_tests webdav_copy_accepts_relative_destination_path
cd backend && RUST_TEST_THREADS=1 cargo test --test handler_webdav_tests
```

Targeted fulltext/OCR:

```bash
cd backend && cargo test --test fulltext_search_tests
```

Auth test stability:

```bash
cd backend && cargo test --test service_auth_tests
```

Final backend gate:

```bash
cd backend && cargo fmt --all -- --check
cd backend && cargo clippy --all-targets --all-features -- -D warnings
cd backend && cargo test --all-features
```

Frontend gate only if frontend types/UI are touched:

```bash
cd frontend && npm run test -- fileListService OcrStatusSection SettingsPageRegression
cd frontend && npm run lint
cd frontend && npm run build
```

## Rollback Plan

- Keep each phase reviewable and independently revertible.
- Do not remove historical docs or unrelated dirty changes.
- Never commit `backend/search-index/`.
- If WebDAV behavior regresses, revert the specific API module/service change and rerun `handler_webdav_tests`.
- If fulltext/OCR indexing regresses, revert `FulltextIndexer` or OCR helper changes and rerun `fulltext_search_tests`.

## Completion Evidence

The implementation is considered complete only when:

- WebDAV module boundaries match this plan.
- `task_queue.rs` delegates indexing to `FulltextIndexer`.
- OCR remains facade-compatible and split into image/pdf/runtime helpers.
- Docs and constraints are updated.
- Final backend gate passes.
