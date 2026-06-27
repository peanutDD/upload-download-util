## CLI 与输出契约

该文档已迁移到 [references/cli.md](references/cli.md)。

当前 CLI 参考包含：

- `pr-auto-fix` / `auto-fix-local` 自动修复流水线
- `review-to-json` 确定性 Review Markdown → JSON 主输入转换
- `doctor` 本机安装、PATH、依赖与源码新鲜度诊断
- `tools/review_to_json.sh` 兼容 shell 入口，委托给 `review-to-json`
- `--review-json`：结构化 review JSON 文件（优先主输入）
- `--review-text`：通用非结构化 Review 文本输入，`--gemini-review` 为旧兼容入口
