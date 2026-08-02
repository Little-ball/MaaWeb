# MaaWeb

MaaWeb 是 [MaaAssistantArknights](https://github.com/MaaAssistantArknights/MaaAssistantArknights)（MAA，明日方舟小助手）的 Web UI 控制端。它作为一个独立壳层，动态加载官方 MaaCore 运行时（`libMaaCore.so`），通过 HTTP API + WebSocket 让浏览器实时控制 MAA 完成任务（刷图、基建、公招等）。

**Linux 部署 · WebUI 控制 · 局域网 ADB 连接远程设备**

## 架构

```
┌─────────────────┐    HTTP/WS    ┌───────────────────┐    ADB over TCP    ┌──────────────┐
│  Web 前端 (Vue3)│ ────────────► │  MaaWeb 服务端     │ ─────────────────► │  Android 设备 │
└─────────────────┘               │  (Rust + axum)     │                    └──────────────┘
                                  │   │                │
                                  │   ▼                │
                                  │  libMaaCore.so     │
                                  │  (官方运行时)       │
                                  └───────────────────┘
```

- **服务端**（`server/`）：Rust + axum。动态加载官方 `libMaaCore.so`，不编译链接 MaaCore，因此构建时无需 MaaCore 源码。
- **前端**（`web/`）：Vue 3 + Vite。设备连接、任务配置、实时日志。
- **设备**：任何通过局域网 ADB 可达的 Android 设备（手机无线调试、平板、模拟器机器）。

## 快速开始

### 1. 准备 MaaCore 运行时

从 [MAA Releases](https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases) 下载 Linux 版（如 `MAA-v*-linux-aarch64.tar.gz` 或 x86_64 版），解压后放到 `server/core_runtime/`：

```
core_runtime/
├── libMaaCore.so          # 核心库
├── resource/              # 资源（模板、图片）
└── Python/                # (可选) Python 绑定
```

> 开发环境用 x86_64 版即可；目标 arm64 设备部署时换 aarch64 版，代码无需改动。

### 2. 构建并运行服务端

```bash
cd server
cargo build --release
./target/release/maaweb-server \
    --core-lib core_runtime/libMaaCore.so \
    --resource-dir core_runtime/resource \
    --web-dir ../web/dist \
    --bind 0.0.0.0:8080
```

### 3. 构建前端

```bash
cd web
npm install        # 或 pnpm install
npm run build      # 输出到 web/dist
```

### 4. 浏览器访问

打开 `http://<服务器IP>:8080`，填写 ADB 路径与设备地址（如 `192.168.1.100:5555`）连接设备，添加任务并开始。

## HTTP API

| 方法 | 路径 | 说明 |
|------|------|------|
| GET  | `/api/version` | MaaCore 版本 |
| GET  | `/api/status`  | 连接/运行状态 |
| POST | `/api/connect` | 连接 ADB 设备（`{adb_path, address, config}`）|
| POST | `/api/task`    | 添加任务（`{task_type, params}`）|
| POST | `/api/start`   | 开始任务队列 |
| POST | `/api/stop`    | 停止任务 |
| POST | `/api/back-home` | 返回游戏首页 |
| WS   | `/api/ws`      | 实时事件流 |

## 支持的任务类型

`Fight`（刷图）、`Infrast`（基建）、`Recruit`（公招）、`Mall`（商店）、`Award`（领取奖励）、`Roguelike`（肉鸽）、`Copilot`（自动作战）等，参数格式见 [MAA 任务参数文档](https://docs.maa.plus/zh-cn/protocol/integration.html)。

## 项目状态

当前为最小可行闭环（MVP）：已实现 MaaCore 动态加载、ADB 连接、任务增删与启动/停止、WebSocket 实时日志、Vue3 前端。后续可扩展：多设备管理、任务计划调度、掉落物统计、Docker 部署等。

## License

[AGPL-3.0](LICENSE)。基于 MaaAssistantArknights（AGPL-3.0）衍生。
