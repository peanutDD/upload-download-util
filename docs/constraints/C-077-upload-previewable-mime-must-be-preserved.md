# C-077: Uploads Must Preserve Previewable MIME Types

普通文件上传不能只依赖客户端上报的 `Content-Type`。Finder、WebDAV、拖拽、系统文件选择器或浏览器兼容层可能把图片、PDF、音视频、文本文件上报为空或 `application/octet-stream`。

约束：
- 前端上传前必须使用统一的扩展名兜底 MIME 推断，并让普通 multipart 上传携带该类型。
- 后端所有写入文件的上传入口（multipart、WebDAV PUT 等）在收到空 MIME 或 `application/octet-stream` 时，必须按文件名扩展名再次兜底推断。
- 后端预览/下载响应对历史 `application/octet-stream` 元数据也必须按文件名扩展名兜底返回有效 `Content-Type`。
- 前端预览类型判断对历史 `application/octet-stream` 元数据也必须按文件名扩展名兜底判断是否可预览。
- 预览支持判断依赖数据库 `mime_type`，所以可预览文件不得因上传入口差异落库为通用二进制类型。

验证：
- 前端服务测试覆盖 octet-stream Finder 文件上传时的 FormData 文件类型。
- 后端 handler 测试覆盖 octet-stream multipart 文件按扩展名落库为可预览 MIME。
- WebDAV handler 测试覆盖 Finder/WebDAV PUT 的 octet-stream 文件按扩展名落库。
- API preview 测试覆盖历史 octet-stream 元数据按扩展名返回可预览 Content-Type。
