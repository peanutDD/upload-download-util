# Finder Upload Preview MIME Fix

日期：2026-05-13

## 目标

修复网页端通过 Finder/文件选择器或 macOS Finder WebDAV 上传的可预览文件无法预览的问题，并兼容已经落库为 `application/octet-stream` 的旧文件。

## 假设

- 预览弹窗根据文件元数据 `mime_type` 判断是否支持图片、PDF、文本、视频、音频预览。
- Finder/浏览器可能把部分本地文件的 multipart 或 WebDAV PUT `Content-Type` 上报为空或 `application/octet-stream`。
- 旧文件已经落库为 `application/octet-stream` 时，前端和后端 preview 都需要运行时兜底，否则用户仍然会看到不能预览。
- 前端已有扩展名兜底 MIME 推断能力，可复用。
- 后端已有 `mime_guess` 依赖，可作为服务端兜底。

## 风险

- 只修 multipart 不能覆盖 Finder WebDAV PUT。
- 只修上传入口不能覆盖已经落库的旧 `application/octet-stream` 元数据。
- MIME 兜底只应在空值或 `application/octet-stream` 时触发，避免覆盖明确的客户端类型。

## 依赖

- `frontend/src/utils/uploadValidation.ts`
- `frontend/src/services/fileUploadService.ts`
- `backend/src/handlers/files/upload.rs`
- `backend/src/api/webdav.rs`
- `backend/src/handlers/files/download/mod.rs`
- `backend/src/utils/mime.rs`
- `frontend/src/utils/mimeType.ts`
- 现有前端 Vitest 与后端 handler 集成测试。

## 步骤

1. 新增前端红灯测试：octet-stream 的 `.png` 普通上传应携带 `image/png`。
2. 新增后端红灯测试：octet-stream multipart 的 `.png` 应返回 `image/png` 元数据。
3. 前端普通上传将文件包装为带推断 MIME 的 `File`。
4. 后端普通上传与 WebDAV PUT 在空 MIME/octet-stream 时按文件名推断。
5. 前端预览类型判断兼容历史 octet-stream 元数据。
6. 后端 preview/download 响应兼容历史 octet-stream 元数据。
7. 运行定向测试、格式化、lint/build 验证。
