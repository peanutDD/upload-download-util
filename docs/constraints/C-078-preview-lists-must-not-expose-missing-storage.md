# C-078: Preview Lists Must Not Expose Missing Storage Objects

文件列表、WebDAV PROPFIND、预览预加载等入口不能把不可读取的本地存储记录暴露给客户端。否则前端会拿到看似有效的 file id，随后 `/preview` 反复 404，用户看到“文件存在但全部不能预览”。

约束：
- macOS Finder/WebDAV 产生的 AppleDouble 资源叉文件（文件名以 `._` 开头）不是用户内容，上传入口必须忽略，列表查询也必须隐藏历史记录。
- 本地存储后端的列表响应必须在返回前验证物理对象仍可打开；缺失对象只记录告警，不返回给普通文件列表或 WebDAV PROPFIND。
- 列表缓存 key 必须随可见性语义变化升级，避免旧缓存继续返回已隐藏的孤儿记录。
- 预览 404 的根因应优先追踪到 DB 记录、存储路径和物理对象一致性，而不是只改前端 catch。

验证：
- WebDAV handler 测试覆盖 `._photo.jpg` PUT 返回成功但不落库、不出现在 PROPFIND。
- API 文件列表测试覆盖本地物理文件缺失时，DB 记录仍存在但不会返回给 `/api/v1/files`。
