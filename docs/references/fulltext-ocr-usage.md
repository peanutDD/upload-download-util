# 全文搜索与 OCR 使用说明

最后更新：2026-05-23

## 概述

本项目的全文搜索由本地 Tantivy 索引提供，不是数据库 `ILIKE` 查询。OCR 不是一个独立的用户操作按钮，而是在文件入库后的后台索引流程中，把图片和扫描 PDF 的识别文本写入全文索引。

核心链路：

```text
上传 / 恢复文件
  -> 入队 search_index_file
  -> worker 读取文件字节
  -> FileContentExtractor 提取普通文本
  -> OcrExtractor 提取图片 / 扫描 PDF 文本
  -> SearchIndexService 写入 Tantivy
  -> 前端搜索框或 API 查询全文索引
```

## 快速使用

### 前端使用

文件列表页的搜索框只要输入非空关键词，前端会自动走全文搜索接口。

前端入口：

- `frontend/src/services/fileListService.ts`
- 当 `query.search.trim()` 非空时，请求 `/api/files/search/fulltext`
- 返回结果会附加到文件对象上：`search_snippet`、`match_source`、`search_score`

### API 使用

```bash
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:3000/api/files/search/fulltext?q=invoice&limit=20"
```

等价 v1 路径：

```bash
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:3000/api/v1/files/search/fulltext?q=invoice&limit=20"
```

参数：

| 参数 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `q` | 是 | 无 | 搜索关键词，空字符串会返回校验错误 |
| `limit` | 否 | `20` | 返回数量，范围 `1..100` |
| `folder_id` | 否 | 无 | 限定文件夹 |
| `mime_type` | 否 | 无 | 限定 MIME，例如 `text/plain`、`application/pdf` |

响应示例：

```json
{
  "query": "invoice",
  "count": 1,
  "index_status": "ready",
  "files": [
    {
      "file": {},
      "score": 1.23,
      "snippet": "matched text around keyword",
      "match_source": "content"
    }
  ]
}
```

`match_source` 取值：

| 值 | 含义 |
| --- | --- |
| `filename` | 命中文件名 |
| `content` | 命中普通文本提取内容 |
| `ocr` | 命中 OCR 识别文本 |
| `category` | 命中文件分类 |

`index_status` 取值：

| 值 | 含义 |
| --- | --- |
| `ready` | Tantivy 索引有命中结果 |
| `fallback` | Tantivy 无命中，回退到文件名搜索 |

回退搜索只匹配当前文件夹内的 `original_filename`，不会同步读取文件内容。搜索摘要 `snippet` 按 UTF-8 字符边界生成，并限制在 160 个字符以内。

## 全文搜索原理

### 索引结构

Tantivy 文档字段：

| 字段 | 用途 |
| --- | --- |
| `file_id` | 文件 ID，用于 upsert/remove 和物化文件信息 |
| `user_id` | 用户隔离，查询时必须作为 Must 条件 |
| `filename` | 文件名全文搜索 |
| `path` | 文件路径/展示路径 |
| `extracted_text` | 普通内容提取文本 |
| `ocr_text` | OCR 识别文本 |
| `category` | 文件分类 |
| `mime_type` | MIME 过滤 |

实现位置：

- `backend/src/services/fulltext_search.rs`
- `SearchDocument`
- `SearchHit`
- `SearchIndexService::open_or_create`
- `SearchIndexService::upsert_document`
- `SearchIndexService::remove_document`
- `SearchIndexService::search`

### 查询流程

1. Handler 校验 `q` 和 `limit`。
2. 检查 `FULLTEXT_SEARCH_ENABLED`。
3. 使用共享的 `AppState.search_index` 查询 Tantivy。
4. 查询条件强制包含当前 `user_id`，避免跨用户泄漏。
5. `QueryParser` 在 `filename`、`extracted_text`、`ocr_text`、`category`、`path` 上解析关键词。
6. 按 Tantivy score 排序。
7. 应用 `folder_id` 和 `mime_type` 过滤。
8. 生成 `snippet` 和 `match_source`。
9. 通过 `file_service.get_file` 物化文件信息；如果文件已删除或不可见，会跳过。

### 写入和删除

写入索引是 upsert：

1. 先按 `file_id` 删除旧文档。
2. 再写入新 `SearchDocument`。
3. commit。
4. reload reader。

删除文件、批量删除、彻底删除或其他需要从索引移除的路径，会入队 `search_remove_file`，worker 再调用 `remove_document`。

相关约束：

- `docs/constraints/C-076-search-index-user-isolation.md`
- `docs/constraints/C-085-fulltext-index-writer-shared.md`
- `docs/constraints/C-087-ocr-runtime-pdf-status.md`

## OCR 使用

### 配置

默认应用配置里 OCR 关闭；Docker Compose 中已开启。

```env
FULLTEXT_SEARCH_ENABLED=true
SEARCH_INDEX_PATH=search-index
OCR_ENABLED=true
OCR_TESSERACT_BIN=tesseract
OCR_PDFTOPPM_BIN=pdftoppm
OCR_PDF_MAX_PAGES=5
```

配置入口：

- `backend/src/config/search.rs`
- `backend/src/config/mod.rs`
- `docker-compose.yml`

默认值：

| 配置 | 默认值 | 说明 |
| --- | --- | --- |
| `FULLTEXT_SEARCH_ENABLED` | `true` | 是否启用全文搜索 |
| `SEARCH_INDEX_PATH` | `search-index` | Tantivy 索引目录 |
| `OCR_ENABLED` | `false` | 是否启用 OCR |
| `OCR_TESSERACT_BIN` | `tesseract` | Tesseract 命令路径 |
| `OCR_PDFTOPPM_BIN` | `pdftoppm` | Poppler `pdftoppm` 命令路径 |
| `OCR_PDF_MAX_PAGES` | `5` | 扫描 PDF 最多 OCR 前 N 页 |

### 状态检查

```bash
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:3000/api/files/search/ocr/status"
```

等价 v1 路径：

```bash
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:3000/api/v1/files/search/ocr/status"
```

响应示例：

```json
{
  "enabled": true,
  "pdf_max_pages": 5,
  "tesseract": {
    "bin": "tesseract",
    "available": true
  },
  "poppler": {
    "bin": "pdftoppm",
    "available": true
  }
}
```

前端设置页会展示 OCR、Tesseract、Poppler、PDF OCR 页数上限。

相关文件：

- `frontend/src/services/ocrStatus.ts`
- `frontend/src/hooks/useOcrStatus.ts`
- `frontend/src/components/settings/OcrStatusSection.tsx`

## OCR 原理

OCR 实现入口：

- `backend/src/services/ocr.rs`
- `OcrExtractor::extract_with_options`

候选文件：

| 类型 | 行为 |
| --- | --- |
| `image/*` | 直接走 Tesseract |
| `png` / `jpg` / `jpeg` / `tif` / `tiff` / `bmp` / `webp` | 直接走 Tesseract |
| `application/pdf` 或 `.pdf` | 先用 `pdftoppm` 转图片，再逐页走 Tesseract |
| 其他文件 | 返回 `unsupported`，不做 OCR |

图片 OCR 命令：

```text
tesseract <input> stdout -l eng
```

PDF OCR 命令：

```text
pdftoppm -png -r 200 -f 1 -l <OCR_PDF_MAX_PAGES> <pdf> <output_prefix>
```

PDF 处理步骤：

1. 把 PDF 写入临时目录。
2. 使用 `pdftoppm` 将前 N 页渲染为 PNG。
3. 对每张 PNG 调用 Tesseract。
4. 拼接每页识别文本，用换行连接。
5. 把 OCR 文本写入 `SearchDocument.ocr_text`。

当前 OCR 语言固定为 `eng`。中文扫描件如果要有较好效果，需要后续支持语言配置和安装对应语言包，例如 `tesseract-ocr-chi-sim`。

OCR 状态：

| 状态 | 含义 |
| --- | --- |
| `disabled` | `OCR_ENABLED=false` |
| `unsupported` | 文件类型不是 OCR 候选 |
| `dependency_missing` | `tesseract` 或 `pdftoppm` 不存在 |
| `failed` | 命令存在但执行失败或无有效文本 |
| `completed` | 成功提取文本 |

OCR 是后台索引的 best-effort 子步骤。`disabled`、`unsupported`、`dependency_missing` 等状态会通过日志和 `fulltext_ocr_status_total` 指标可观测，但不会让 `search_index_file` worker 失败；文件仍会用文件名和可提取普通文本写入全文索引。

## 普通文件内容提取范围

普通内容提取入口：

- `backend/src/services/file_content_extractor.rs`

支持情况：

| 类型 | 状态 |
| --- | --- |
| `text/*` | 支持 |
| Markdown | 支持 |
| JSON | 支持 |
| XML | 支持 |
| CSV / log | 支持，按扩展名推断 |
| PDF 文本层 | 支持，走 `pdf-extract` |
| DOCX | 支持，走 `docx_lite` |
| HTML | 支持，简单去标签 |
| 图片 | 不走普通提取，只靠 OCR |
| 扫描 PDF | 普通提取通常为空或失败，只靠 OCR |
| DOC | 暂不支持 |
| XLSX / PPTX | 当前返回空字符串，未真正实现 |
| 压缩包 | 不做内容索引 |

## 后台任务和 worker

全文搜索索引不是上传请求同步完成的。上传、恢复、删除只负责入队，真正索引由 worker 消费。

启动 worker：

```bash
cd /Users/tyone/github/upload-download-util/backend
cargo run --bin worker
```

相关任务：

| 任务 | 触发场景 | 行为 |
| --- | --- | --- |
| `search_index_file` | 上传、秒传、恢复、WebDAV 写入 | 读取文件、提取文本、OCR、写入 Tantivy |
| `search_remove_file` | 删除、批量删除、彻底删除等 | 从 Tantivy 移除文件文档 |

任务去重 key：

```text
search:<file_id>
```

worker 关键文件：

- `backend/src/bin/worker.rs`
- `backend/src/services/task_queue.rs`
- `backend/src/services/fulltext_indexer.rs`
- `backend/src/services/file/upload.rs`
- `backend/src/services/file/delete.rs`

## 重构后的模块边界

WebDAV API 已按协议 helper 和业务 service 拆分：

| 边界 | 责任 |
| --- | --- |
| `backend/src/api/webdav/mod.rs` | `/dav` 路由入口、HTTP method dispatch、OPTIONS、统一错误映射、请求指标 |
| `backend/src/api/webdav/auth.rs` | Basic/Bearer token 认证适配、WebDAV root folder 校验 |
| `backend/src/api/webdav/methods.rs` | MKCOL/PUT/GET/HEAD/DELETE/MOVE/COPY 的 HTTP header/body 读取和 response 构造 |
| `backend/src/api/webdav/propfind.rs` | PROPFIND 请求解析、Depth 遍历、Multi-Status XML 生成 |
| `backend/src/api/webdav/lock.rs` | Class 2 LOCK/UNLOCK、active lock discovery、锁冲突查询 |
| `backend/src/api/webdav/path.rs` | DAV path 清洗、Destination 兼容解析、href 编码 |
| `backend/src/api/webdav/range.rs` | 单段 `Range` 解析 |
| `backend/src/api/webdav/lock_headers.rs` | `Lock-Token` / `If` token 解析 |
| `backend/src/api/webdav/xml_fragments.rs` | DAV XML fragment 构造和 XML 转义 |
| `backend/src/services/webdav.rs` | 资源解析、复制/删除树、cache bump、fulltext 入队等业务副作用 |

全文搜索和 OCR 已按执行职责拆分：

| 边界 | 责任 |
| --- | --- |
| `backend/src/services/task_queue.rs` | 后台任务 dequeue、retry、mark succeeded/failed |
| `backend/src/services/fulltext_indexer.rs` | `search_index_file` 的业务编排：读文件、抽文本、OCR、写 Tantivy |
| `backend/src/services/fulltext_search.rs` | Tantivy index schema、writer、query |
| `backend/src/services/fulltext_search/snippet.rs` | snippet 和 match source 判定 |
| `backend/src/services/ocr.rs` | `OcrExtractor` facade 和类型 |
| `backend/src/services/ocr/image.rs` | 图片 OCR 临时文件处理 |
| `backend/src/services/ocr/pdf.rs` | PDF page render + per-page OCR |
| `backend/src/services/ocr/runtime.rs` | Tesseract runtime 调用和 dependency 检测 |

## 排查清单

### 搜不到新上传文件内容

检查：

1. `FULLTEXT_SEARCH_ENABLED=true`
2. worker 是否已启动：`cargo run --bin worker`
3. `background_tasks` 中是否有 `search_index_file`
4. 搜索结果 `index_status` 是否为 `fallback`
5. 文件类型是否属于普通提取或 OCR 支持范围

### 图片或扫描 PDF 搜不到文字

检查：

1. `OCR_ENABLED=true`
2. `/api/files/search/ocr/status` 中 `tesseract.available=true`
3. PDF OCR 还需要 `poppler.available=true`
4. `OCR_PDF_MAX_PAGES` 是否覆盖目标页码
5. 当前 OCR 语言是 `eng`，中文扫描件默认效果有限

### 搜索只命中文件名

常见原因：

1. worker 未消费索引任务，接口进入 fallback。
2. 文件类型不支持内容提取。
3. OCR 关闭或依赖缺失。
4. 扫描 PDF 的目标文字不在前 `OCR_PDF_MAX_PAGES` 页。

### 多实例或生产部署注意

Tantivy 索引是本地目录。当前实现适合单应用实例共享一个本地索引目录的场景；多实例部署时，需要额外设计共享索引、集中式搜索服务或重建策略。

## 代码索引

| 主题 | 文件 |
| --- | --- |
| WebDAV API 入口 | `backend/src/api/webdav/mod.rs` |
| WebDAV 业务服务 | `backend/src/services/webdav.rs` |
| 全文搜索 API | `backend/src/handlers/files/fulltext_search.rs` |
| Tantivy 索引服务 | `backend/src/services/fulltext_search.rs` |
| 全文索引编排 | `backend/src/services/fulltext_indexer.rs` |
| OCR 提取 | `backend/src/services/ocr.rs` |
| 普通内容提取 | `backend/src/services/file_content_extractor.rs` |
| 全文索引 worker | `backend/src/services/task_queue.rs` |
| worker 进程 | `backend/src/bin/worker.rs` |
| 上传入队 | `backend/src/services/file/upload.rs` |
| 删除/恢复入队 | `backend/src/services/file/delete.rs` |
| API 路由 | `backend/src/api/files.rs` |
| 应用状态共享索引 | `backend/src/state.rs` |
| 前端全文搜索调用 | `frontend/src/services/fileListService.ts` |
| 前端 OCR 状态服务 | `frontend/src/services/ocrStatus.ts` |
| 设置页 OCR 状态 UI | `frontend/src/components/settings/OcrStatusSection.tsx` |
