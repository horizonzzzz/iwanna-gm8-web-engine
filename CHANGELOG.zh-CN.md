# 更新日志

本文件记录项目的重要变更。

English version: [CHANGELOG.md](CHANGELOG.md)

## [Unreleased]

暂无更新。

## [0.2.0-beta.2] - 2026-07-15

### 变更

- 发布自动化现在会把中英文更新日志一并写入 GitHub Release。
- 不再跟踪机器相关的 assistant skill 和 hook 配置；本地 `.claude/`、`.codex/` 目录已加入忽略。

### 修复

- 生成 package 时归一化缺失的可见房间背景和 tile 引用，并通过明确警告保留严格校验。

## [0.2.0-beta.1] - 2026-07-14

### 新增

- 为受支持的 GM8 风格 IWanna 游戏提供浏览器上传到 Canvas 游玩流程。
- 增加 Rust detector、parser、package validator、上传 API、生成包服务和健康检查接口。
- 增加 WASM-first runtime bridge，并保留 `/shell` 诊断界面。
- 增加检测、生成与校验 package、sample audit 和 runtime diagnostics CLI 工作流。
- 增加单容器 Docker 发布路径，支持发布 `linux/amd64` 和 `linux/arm64` 多架构镜像。
- 增加 runtime package v1，包含标准化资源、房间/对象数据、原始与 lowered logic、兼容性分析和跨文件校验。

### 变更

- 生成包通过校验后才会发布，避免未验证的 package 进入服务目录。
- 公共 `/` 页面在 package、WASM 或 runtime 出错时 fail closed；静态 fallback 仅保留在 `/shell`。
- 兼容性明确报告为 `supported`、`partial` 或 `blocked`，不再暗示支持所有 GM8 游戏。
- runtime 已覆盖当前 IWanna 关键移动、碰撞、生命周期、房间、音频、存档点和绘制路径，同时记录剩余 GM8 缺口。

### 已知限制

- 本项目是经过筛选的 GM8 兼容性 Beta，不是通用 GM8 模拟器。
- 更广泛的 GML/GM8 语义、鼠标输入、高级绘制、多视图 camera、完整文件/音频行为以及 DLL/external 调用仍不完整或不受支持。
- 包含 OpenGMK `gm8exe` 的发布产物仍需遵守其 GPL-2.0-only 对应源码义务。
