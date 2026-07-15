# iwanna-gm8-web-engine

[English README](README.md)

面向旧版 GM8 风格 IWanna 游戏的浏览器运行引擎。当前发布线为
`0.2.0-beta.1`。

[更新日志](CHANGELOG.md) · [中文更新日志](CHANGELOG.zh-CN.md)

> [!IMPORTANT]
> Beta 表示“上传到 Canvas 游玩”的产品链路已经形成，不表示任意 GM8
> 游戏都能完整兼容或通关。

## Beta 产品形态

部署后只有两个页面：

- `/`：用户页，只做 `.exe` / `.zip` 上传、处理状态、Canvas 游玩和重置
- `/shell`：保留的开发诊断 Shell，用于 package 检查、选房间和运行时诊断

单容器主流程：

```text
上传 -> iwm-api -> detector -> parser -> runtime-v1 validator
     -> /data/games/<sha256> -> WASM runtime -> Canvas
```

Rust 服务不会执行上传的 EXE 或 DLL。它只解析受支持的 GM8 数据；生成包
通过结构校验后才会发布给浏览器。

## 运行 Beta

```powershell
git submodule update --init --recursive
.\scripts\build-beta.ps1
docker run --rm --name iwm-beta -p 3000:3000 -v iwm-data:/data iwm-beta:0.2.0-beta.1
```

打开 `http://127.0.0.1:3000`。健康检查为 `/healthz`。

打包脚本会先依次拉取 Rust、Node、Debian 三个基础镜像，再执行
`docker build`，避免当前网络环境下 BuildKit 并行鉴权失败。

Docker volume 只保存生成后的 runtime package。上传原文件位于不会被静态
服务暴露的 staging 目录，请求结束后删除。生成包保留 24 小时，最多 16 个。

## 本地开发

```powershell
git submodule update --init --recursive
npm --prefix runtime install
rustup target add wasm32-unknown-unknown
cargo build -p iwm-runtime-web --release --target wasm32-unknown-unknown
npm --prefix runtime run sync:wasm
npm --prefix runtime run build
cargo run -p iwm-api
```

另开终端：

```powershell
npm --prefix runtime run dev -- --host 127.0.0.1
```

Vite 使用 `http://127.0.0.1:4173`，并把 `/api`、`/games` 代理到 3000 端口。

## API

| 方法 | 路径 | 用途 |
| --- | --- | --- |
| `POST` | `/api/v1/games` | multipart 上传，字段名为 `game` |
| `GET` | `/games/{sha256}/*` | 已校验的生成包与资源 |
| `GET` | `/healthz` | 服务健康状态与版本 |

当前边界：单次上传 512 MiB；ZIP 最多 4,096 项，单项解压 512 MiB，总解压
与生成包 1 GiB；同时只运行一个 parser；HTTP 解析窗口 120 秒。ZIP 路径穿越
和特殊文件会直接拒绝。

## 验证

```powershell
cargo test
cargo clippy -p iwm-api -p iwm-detector -p iwm-parser --all-targets --locked --no-deps -- -D warnings
npm --prefix runtime test
npm --prefix runtime run build
npm --prefix runtime run test:browser
docker build -t iwm-beta:0.2.0-beta.1 .
```

浏览器和 sample 验证必须先重建 release WASM，并用当前 parser 重新生成
package；`runtime/public/` 中的旧文件不能作为 release 证据。

## 兼容范围

当前 runtime 已覆盖 Gap 文档中验证过的 IWanna 关键移动、碰撞、生命周期、
存档点、房间、音频和绘制路径，但仍不是完整 GM8 运行器。主要缺口包括更广
的 GML/GM8 语义、鼠标、完整 Draw、逐帧 mask、多 view/camera、完整音频与
文件 API，以及 DLL/external 调用。

API 只会报告 `supported`、`partial` 或 `blocked`，不会承诺任意上传都能玩。

## 项目结构

- `crates/iwm-api/`：上传 API、生成包和静态 Web 服务
- `crates/iwm-detector/`：安全解压、目录清单和引擎判定
- `crates/iwm-parser/`：GM8 解析、资源导出和 logic lowering
- `crates/iwm-runtime-model/`：package schema 与校验
- `crates/iwm-runtime-host/`：项目自有 host boundary
- `crates/iwm-runtime-core/`：确定性 runtime 行为
- `crates/iwm-runtime-web/`：浏览器/WASM bridge
- `crates/iwm-cli/`：开发诊断与 sample 工作流
- `runtime/`：用户 Web、`/shell`、输入/音频/渲染 glue
- `docs/`：当前 note 和明确标记为历史的设计记录

本地有版权的 sample 只能放在 `samples/local/iwanna-examples/`，不得提交。
`IWBT_Dife` 仍是 L1 回归样本，ArioTrials 是当前 L2 开发样本。

## 当前文档

- `CHANGELOG.md`、`CHANGELOG.zh-CN.md`
- `docs/notes/beta-release.md`
- `docs/notes/package-format-v1-runtime.md`
- `docs/notes/runtime-wasm-gap-analysis.md`
- `docs/notes/runtime-performance-optimization.md`
- `docs/notes/runtime-gold-sample.md`
- `docs/notes/runtime-vendor-reference-map.md`
- `docs/notes/opengmk-host-coupling-audit.md`
- `docs/notes/testing-strategy.md`

`docs/superpowers/specs/` 中原始 MVP 设计只保留作历史基线，不再是当前部署规范。

## 许可证

项目自有源码使用 MIT。parser/API 二进制链接声明为 `GPL-2.0-only` 的
OpenGMK `gm8exe`；发布 API 二进制或 Docker 镜像时，必须履行 `NOTICE.md`
中说明的 GPL 对应源码义务。发布前请同时阅读 `vendor/README.md`。
