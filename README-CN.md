# iwanna-gm8-web-engine

[English README](README.md)

面向旧版 GM8 风格 IWanna fangame 的浏览器可玩 MVP。

> [!NOTE]
> 本项目当前不是完整 GM8 运行器。它聚焦主流旧版 GM8 风格 IWanna 游戏，目标是把可支持的原始游戏包解析为项目自有运行时包，并通过浏览器 WASM 路径执行。

## 当前方向

当前主流程是：

1. 检测上传的游戏包是否像可支持的 GM8 风格目标
2. 使用后端/工具链 crate 解析可支持包
3. 归一化为项目自有 runtime package
4. 通过浏览器面对的 WASM runtime path 执行

Phase 4 正在推进中。浏览器 shell 现在主要承担 package loader、检查器、诊断界面和 host bridge 的职责。长期运行方向是 WASM-first runtime path；此前的 TypeScript gameplay runtime 不是长期引擎方向。

当前已实现的主要部分：

- `crates/iwm-detector/`：识别可能支持的目标游戏包
- `crates/iwm-parser/`：读取 GM8 资源并构建归一化 runtime package
- `crates/iwm-cli/`：提供检测、构建、校验和 runtime diagnostics 命令
- `crates/iwm-runtime-model/`：共享 package schema 和校验
- `crates/iwm-runtime-host/`：runtime host boundary trait 和默认/headless helper
- `crates/iwm-runtime-core/`：确定性 runtime-core 行为和当前 lowered-logic 执行切片
- `crates/iwm-runtime-web/`：浏览器可加载 WASM bridge
- `runtime/`：加载 package、驱动 WASM bridge、转发输入、渲染 frame command、展示诊断信息

runtime package 当前包含保留的 raw logic、结构化 lowered logic、浏览器可用资源、sprite 碰撞边界/mask、GM font atlas 元数据等。当前 lowered runtime path 覆盖 IWanna 关键子集，但不是完整 GML/GM8 语义实现。

## 快速开始

```powershell
git submodule update --init --recursive
npm --prefix runtime install
rustup target add wasm32-unknown-unknown
```

在 Windows 上构建 WASM target 时，需要从 Visual Studio Developer Command Prompt 执行，或确保 `clang` 和 `clang++` 在 `PATH` 中。

## 验证

```powershell
cargo test
npm --prefix runtime test
npm --prefix runtime run test:browser
npm --prefix runtime run build
```

默认 Rust 测试使用内存 fixture。本地 sample 支持的 runtime-core 测试需要显式 feature，避免陈旧的本地生成包影响普通验证：

```powershell
cargo test -p iwm-runtime-core --features local-sample-tests
```

仅在重新构建并校验 `runtime/public/packages/sample` 后运行这个 feature。

## 构建 WASM Bridge

```powershell
$env:PATH='C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\Llvm\bin;' + $env:PATH
$env:CC='clang'
$env:CXX='clang++'
cargo build -p iwm-runtime-web --target wasm32-unknown-unknown
npm --prefix runtime run sync:wasm
```

`sync:wasm` 会把：

```text
target\wasm32-unknown-unknown\debug\iwm_runtime_web.wasm
```

复制到：

```text
runtime\public\wasm\iwm_runtime_web.wasm
```

## 构建 Runtime Package

```powershell
cargo run -p iwm-cli -- detect --input C:\path\to\game
cargo run -p iwm-cli -- build-package --input C:\path\to\game --output .\runtime\public\packages\sample
cargo run -p iwm-cli -- validate-package --input .\runtime\public\packages\sample
```

浏览器 shell 默认 package path 是 `/packages/sample`，对应本地目录：

```text
runtime\public\packages\sample\
```

`validate-package` 会在浏览器 smoke 前检查归一化 runtime package contract，包括 manifest 计数、稀疏 id 引用、资源引用，以及 `scripts.ir.json`、`logic.raw.json`、`logic.lowered.json` 之间的 logic block 一致性。

## Runtime Diagnostics

package 校验通过后，使用 CLI diagnostics 运行 headless runtime，并按实际 lowered execution 路径排序 blocker：

```powershell
cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --ticks 600
cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --select-room 143 --ticks 240 --press-keys 16
cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --input-script .\runtime-input-script.json --trace-player --trace-every 1
```

常用参数：

- `--select-room <room_id>`：tick 前进入指定 room
- `--preselect-ticks <n>`：手动选 room 前先推进 boot room
- `--ticks <n>`：诊断 tick 窗口
- `--press-keys`、`--hold-keys`、`--input-script`：驱动虚拟键输入
- `--trace-player`：输出 compact player 行为 trace
- `--trace-output <path>`：写出完整 diagnostics JSON

诊断 JSON 包含 runtime blocker 聚合、runtime lifecycle event，以及可选 player trace summary。已提交的输入脚本示例位于：

```text
docs/notes/runtime-scenarios/
```

## 运行浏览器 Shell

```powershell
npm --prefix runtime run dev -- --host 127.0.0.1
```

然后打开：

```text
http://127.0.0.1:4173
```

当前 shell 行为：

- 从 package path 输入加载归一化 package
- WASM bridge 可用时启动 WASM runtime
- WASM 缺失或 boot 失败时回退到静态 package inspection
- 转发原始 GM virtual-key hold/press/release 状态
- 按当前 room speed 自动 tick WASM path，并提供 Pause/Resume
- 在 canvas 上渲染 runtime frame
- 暴露 HUD telemetry 和 copy-first 纯文本 runtime report
- 保留 package inspector 作为次要只读 tab

当前手测按键：

- `ArrowLeft` / `A`：向左
- `ArrowRight` / `D`：向右
- `Space` / `ArrowUp` / `W`：跳跃输入
- `R`：原始 package keyboard input
- `Reset` 按钮：显式 shell reset

## 项目结构

- `docs/`：设计文档、状态 note 和项目指导
- `crates/iwm-detector/`：目标检测和 package inventory
- `crates/iwm-parser/`：GM8 解析、package 构建、资源导出、logic lowering
- `crates/iwm-cli/`：开发者 CLI
- `crates/iwm-runtime-model/`：共享 runtime package schema 和校验
- `crates/iwm-runtime-host/`：runtime host boundary 类型和 helper
- `crates/iwm-runtime-core/`：确定性 runtime-core 行为
- `crates/iwm-runtime-web/`：WASM/browser bridge surface
- `runtime/`：浏览器 shell、诊断 UI、package loading 和渲染 glue
- `samples/local/iwanna-examples/`：本地 sample corpus，存在时用于本地验证
- `vendor/`：上游参考 submodule

计划中的后续区域：

- `backend/`

## Sample Corpus

本地 sample corpus 位于：

```text
samples/local/iwanna-examples/
```

当前分类：

- `gm8-core`
- `gm8-extended`
- `needs-manual-check`
- `non-target`

这些分类是开发中的工作标签，不是最终事实。不要把有版权的 sample binary 提交到 git。

默认 gold sample 是：

```text
samples/local/iwanna-examples/gm8-core/IWBT_Dife
```

如果当前机器没有该路径，可以使用最接近的 `gm8-core` sample 做本地 smoke，但这不改变仓库层面的目标样本。

## Vendored References

当前跟踪的参考仓库：

- `vendor/OpenGMK/`
- `vendor/GM8Decompiler/`

用途：

- 研究 GM8 executable handling
- 验证 parser 假设
- 对照 runtime 语义
- 审计 WASM-first host boundary 假设

> [!CAUTION]
> OpenGMK 生态组件可能涉及 `GPL-2.0-only`。任何直接依赖或代码复用都必须是有意识的架构和许可证决策。

## 范围边界

- 聚焦主流旧版 GM8 风格 IWanna fangame
- parser/runtime contract 只在 gold-sample 证据要求时做有目标的扩展
- 浏览器工作聚焦 WASM host integration、diagnostics、controls 和 rendering
- 不重新扩展并行的 TypeScript gameplay runtime
- 不宣称当前 IWanna-critical subset 已具备完整 GM8 parity

## 关键文档

当前优先阅读：

- `README.md`
- `AGENTS.md`
- `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`
- `docs/notes/package-format-v1-runtime.md`
- `docs/notes/runtime-wasm-gap-analysis.md`
- `docs/notes/runtime-gold-sample.md`
- `docs/notes/runtime-vendor-reference-map.md`
- `docs/notes/opengmk-host-coupling-audit.md`
- `docs/notes/testing-strategy.md`

旧设计 spec 可能有历史参考价值，但当它们和当前 note/code 冲突时，以当前 note 和实际代码为准。
