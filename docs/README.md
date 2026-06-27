# upload-download-util 项目文档中心

欢迎来到 upload-download-util 的文档中心。本仓库包含项目的完整技术文档，按类别组织，便于团队成员快速查找和理解项目信息。

---

## 📁 文档结构

```
docs/
├── README.md                          # 本文件 - 文档总览
├── CHANGELOG.md                       # 变更日志（Bug修复、新功能、优化）
├── quality-score.md                   # 质量评分记录
├── NEXT_STEPS.md                      # 路线图与后续计划
├── BACKEND_UPGRADE_2026-04-28.md      # 后端依赖升级详细文档
├── UPGRADE_RECOMMENDATIONS.md         # 升级建议汇总
├── HIGH_CONCURRENCY.md                # 高并发优化完整方案（前端+后端）
├── BILIBILI_TECH_UPGRADE.md           # B站技术实践参考
├── FEATURE_FLAGS.md                   # 功能开关矩阵与接口边界
├── CODE_REVIEW_GUIDE.md               # 代码审查规范（含使用指南与报告模板）
├── constraints/                       # 永久约束规则
│   ├── ai-auto-fix-rules.md           # AI自动修复规则
│   ├── ci-pipeline.md                 # CI流水线约束
│   └── error-boundary-rules.md        # 错误边界规则
├── design-docs/                       # 技术设计文档
│   ├── auto-review.md                 # 自动审查设计
│   ├── auto-review-flow.md            # 自动审查流程
│   ├── auto-review-usage-flow.md      # 自动审查使用流程
│   ├── codex-pr-auto-fix-design.md    # PR自动修复设计
│   ├── codex-cli-toolbox-extraction.md # codex-cli 独立工具箱改造设计
│   ├── upload-completeness-hardening.md # 上传完整性与断点续传设计
│   ├── 2026-05-04-settings-page-optimization.md # Settings 页面优化设计
│   └── frontend-component-split-detail.md # 前端组件拆分详情
├── exec-plans/                        # 执行计划
│   ├── 2026-04-27-error-boundary-unification.md # 错误边界统一计划
│   ├── 2026-04-30-frontend-file-service-split.md # 前端服务拆分计划
│   └── 2026-05-04-settings-page-optimization.md # Settings 页面优化执行计划
├── product/                           # 产品文档
│   ├── COMMERCIALIZATION_PLAN.md      # 商业化计划
│   └── COMMERCIALIZATION_PLAN_RAW.md  # 商业化计划草稿
└── references/                        # 参考资料
    ├── fulltext-ocr-usage.md          # 全文搜索与 OCR 原理、使用清单、排查指南
    └── workflow-integration.md        # 工作流集成参考
```

---

## 🎯 快速导航

### 🏗️ 架构与设计
| 文档 | 描述 |
|------|------|
| `design-docs/frontend-component-split-detail.md` | 前端组件大文件拆分详细设计 |
| `design-docs/2026-05-04-settings-page-optimization.md` | Settings 页面外观、移动端布局、表单样式与回归测试设计 |
| `design-docs/upload-completeness-hardening.md` | 上传 ownership、断点续传、分片完整性与秒传 copy 设计 |
| `design-docs/codex-pr-auto-fix-design.md` | PR自动修复系统设计 |
| `design-docs/codex-cli-toolbox-extraction.md` | codex-cli 独立工具箱改造设计 |
| `design-docs/auto-review.md` | 自动代码审查系统设计 |

### 📊 高并发优化
| 文档 | 描述 |
|------|------|
| `HIGH_CONCURRENCY.md` | 前后端高并发完整优化方案（整合前端减压 + 后端优化） |
| `FEATURE_FLAGS.md` | 功能开关矩阵与接口边界定义 |

### ✅ 代码审查
| 文档 | 描述 |
|------|------|
| `CODE_REVIEW_GUIDE.md` | 代码审查规范指南（含作者/审查人操作步骤、检查清单、报告模板） |

### 📋 开发流程
| 文档 | 描述 |
|------|------|
| `NEXT_STEPS.md` | 项目路线图和后续改进计划 |
| `exec-plans/*.md` | 各阶段执行计划文档 |
| `exec-plans/2026-05-15-webdav-fulltext-ocr-implementation-plan.md` | WebDAV、全文搜索、OCR 的分阶段实现与重构计划 |

### 📈 项目状态
| 文档 | 描述 |
|------|------|
| `CHANGELOG.md` | 变更日志（按时间倒序） |
| `quality-score.md` | 各任务质量评分记录 |

### 🔧 升级与维护
| 文档 | 描述 |
|------|------|
| `BACKEND_UPGRADE_2026-04-28.md` | 后端依赖升级详细指南 |
| `UPGRADE_RECOMMENDATIONS.md` | 升级建议汇总（按优先级排序） |

### 🔎 搜索与 OCR
| 文档 | 描述 |
|------|------|
| `references/fulltext-ocr-usage.md` | 全文搜索与 OCR 的原理、API、配置、worker 使用清单和排查指南 |
| `exec-plans/2026-05-15-webdav-fulltext-ocr-implementation-plan.md` | WebDAV、全文搜索、OCR 实现计划、模块边界、测试门禁 |

### 📜 约束与规范
| 文档 | 描述 |
|------|------|
| `constraints/ai-auto-fix-rules.md` | AI自动修复永久约束 |
| `constraints/ci-pipeline.md` | CI流水线约束规则 |
| `constraints/error-boundary-rules.md` | 错误边界处理规则 |

---

## 🚀 入门指南

### 新开发者
1. 阅读 `NEXT_STEPS.md` 了解项目现状和路线图
2. 查看 `CHANGELOG.md` 了解最近的变更
3. 参考 `HIGH_CONCURRENCY.md` 了解高并发优化策略

### 维护者
1. 更新 `CHANGELOG.md` 记录每次重要变更
2. 更新 `quality-score.md` 记录任务质量评分
3. 在 `exec-plans/` 创建新的执行计划文档

### 架构师
1. 在 `design-docs/` 添加新的技术设计文档
2. 在 `constraints/` 添加永久约束规则
3. 更新 `UPGRADE_RECOMMENDATIONS.md` 评估升级优先级

### 代码审查
1. **作者**：提交 PR 前阅读 `CODE_REVIEW_GUIDE.md`，按模板填写审查类别
2. **审查人**：按 `CODE_REVIEW_GUIDE.md` 中的检查清单执行审查
3. **团队**：首次使用参考 `CODE_REVIEW_GUIDE.md` 的「开始使用」章节配置仓库

---

## 📝 文档规范

### 命名约定
- 使用 **kebab-case**（短横线分隔）
- 日期格式：`YYYY-MM-DD-description.md`
- 保持文件名简洁描述内容

### 内容结构
1. **标题**：清晰描述文档目的
2. **概述**：简要说明内容
3. **正文**：结构化内容（使用 Markdown 标题层级）
4. **验收标准**：可验证的完成标准（如适用）
5. **版本历史**：记录文档更新（如适用）

### 版本控制
- 重要变更更新版本号
- 记录更新日期
- 保持向后兼容

---

## 📋 快速检查清单

### 代码审查（PR 合并前）
- [ ] CI 全部通过（fmt/clippy/test、lint/test/build）
- [ ] 至少 1 人 Approve
- [ ] 无未解决的讨论
- [ ] 无冲突

### 发布前检查
- [ ] 环境变量与配置完整
- [ ] 迁移顺序与依赖正确
- [ ] 存储与备份就绪
- [ ] 健康检查接口可用

---

## 🗑️ 已归档文档

以下文档内容已整合到其他文档中，不再单独维护：

| 原文档 | 整合到 | 说明 |
|--------|--------|------|
| `高并发.md` | `HIGH_CONCURRENCY.md` | 高并发优化方案已整合 |
| `前端降低后端并发压力方案.md` | `HIGH_CONCURRENCY.md` | 前端减压方案已整合 |
| `开关矩阵与接口边界.md` | `FEATURE_FLAGS.md` | 功能开关矩阵已整合 |
| `CODE_REVIEW_START.md` | `CODE_REVIEW_GUIDE.md` | 开始使用指南已整合到 §12 |
| `CODE_REVIEW_USAGE.md` | `CODE_REVIEW_GUIDE.md` | 使用指南已整合到 §13 |
| `CODE_REVIEW_REPORT.md` | `CODE_REVIEW_GUIDE.md` | 报告模板已整合到 §14 |

---

**最后更新**: 2026-05-14
**文档版本**: v2.5
