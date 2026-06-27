# Preview 404 Orphan WebDAV Fix

日期：2026-05-14

## 目标

修复 Finder/WebDAV 上传后网页端文件列表中出现无法预览条目的问题：列表不再暴露 macOS `._*` 资源叉文件，也不再暴露本地存储文件已丢失的孤儿 DB 记录，从源头减少 `/api/files/:id/preview` 404。

## 假设

- 用户控制台里的 `/preview` 404 表示后端找不到对应存储对象，而不是前端 MIME 判断失败。
- 出错 ID 在数据库中仍有 active 记录，但对应 `file_path` 在本地磁盘不存在。
- `._HF...jpeg` 是 macOS AppleDouble 资源叉文件，不应作为用户文件展示或预览。
- 本地存储检查可以放在 service 列表层，既覆盖 API 列表，也覆盖 WebDAV PROPFIND。

## 风险

- 本地列表逐项打开文件会增加少量 I/O；当前页面级列表规模可接受。
- 当前页中过滤孤儿记录后，传统分页 total 只能扣减本页隐藏数，跨页全量 orphan 数仍依赖后续维护任务清理。
- 如果前端已有旧列表缓存，部署后需要重新拉取列表才能消失。
- S3 等远程对象存储不做列表期逐项网络检查，避免列表性能退化。

## 依赖

- `backend/src/api/webdav.rs`
- `backend/src/services/file/list.rs`
- `backend/src/repositories/files.rs`
- `backend/src/utils/mime.rs`
- `backend/tests/handler_webdav_tests.rs`

## 步骤

1. 查询报错 file id 的 DB 元数据和本地 `file_path`，确认 root cause 是 active DB 记录指向缺失本地文件。
2. 新增红灯测试：WebDAV PUT `._photo.jpg` 不应落库或出现在 PROPFIND。
3. 新增红灯测试：删除本地物理对象后，API 文件列表不应返回孤儿记录。
4. WebDAV PUT 对 AppleDouble 文件直接返回成功并忽略。
5. Repository SQL 隐藏历史 AppleDouble 记录。
6. Service 列表层过滤 AppleDouble 与本地缺失对象，并记录可观测日志。
7. 升级文件列表缓存 key，避免旧缓存继续返回不可见记录。
8. 运行 WebDAV handler 集成测试、后端格式化、clippy/check。
