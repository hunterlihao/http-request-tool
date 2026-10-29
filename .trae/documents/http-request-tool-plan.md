# 轻量便携 HTTP 请求工具 — 开发计划

## 一、项目概述（Summary）

开发一个**便携版（Portable）桌面 HTTP 请求调试工具**，类似轻量版 Postman：

- **形态**：Windows 桌面 GUI 应用
- **关键诉求**：单个 `exe`，双击即用，无需安装、无运行时依赖
- **技术栈**：Rust + egui/eframe + reqwest + SQLite，**纯 Rust 绘制，零额外安装**

---

## 二、当前环境分析（Current State Analysis）

### 2.1 目标目录
- 工作目录 `d:\code\workspace\newcode` 当前**为空**，是全新项目，可直接初始化。

### 2.2 本机已确认环境（已完成实测，非假设）

| 项 | 版本 | 结论 |
|----|------|------|
| Rust toolchain | `stable-x86_64-pc-windows-msvc`（rustup 1.29 管理） | ✅ **active + default** |
| cargo | 1.96.0 | ✅ |
| Visual Studio 2022 | Community 版，已含 `VC.Tools.x86.x64` C++ 组件 | ✅ rustc 链接所需 |
| Node | 24.20.0 | 暂不使用 |
| Python | 3.13.13 | 暂不使用 |
| Go | 1.26.4 | 不使用（见选型） |
| GCC/MinGW | ❌ 未安装 | **因此弃用 Fyne/Wails** |
| .NET SDK | ❌ 未安装 | 不使用 |

### 2.3 关键结论
- Rust 是 MSVC host，且 VS2022 C++ 工具链齐全 → **egui 可本地直接编译，无需安装任何额外工具**。
- Rust cargo bin 在 `C:\Users\Hunter\.cargo\bin`，但**未加入默认 PATH**。计划中需处理（项目内使用 `toolexec`/绝对路径编译，或引导用户加 PATH），以保证后续命令行可用。

---

## 三、技术选型与理由（Technology Decision）

| 维度 | 选择 | 理由 |
|------|------|------|
| 语言 | **Rust** | 本机 MSVC 工具链就绪、零依赖编译、性能优越、单二进制分发 |
| GUI | **egui / eframe** | 纯 Rust 软件渲染，无 WebView/无 CGO，单 exe 5~15MB；内置滚动区/可折叠区/文本高亮（响应体格式化展示友好） |
| HTTP | **reqwest**（blocking + 自定义 TLS/代理） | 支持代理、超时、TLS 跳过自签名、重定向、多部分上传；`native-tls` 用系统 Schannel，无需额外证书 |
| 数据持久化 | **rusqlite**（bundle 模式） | 单文件 SQLite，历史/集合/环境变量结构化存储，查询/去重/删除方便；`bundled` 特性将 SQLite 静态编入 exe，避免额外 dll |
| 数据导出 | SQLite 导出 JSON | 实现集合/历史导出导入为 JSON 文件 |
| 打包 | **cargo build --release + winres 图标** | 产出单 exe；可选 `upx`（不强制）进一步压缩体积 |

**弃用说明**：Go+Fyne/Wails 需补装 MinGW（体积大且违背便携初衷）；Electron/Tauri 依赖 WebView2，不够纯粹。Rust+egui 最契合"轻量便携"诉求。

---

## 四、功能范围（Scope）

### 4.1 基础功能（必须有）
1. **方法选择**：GET / POST / PUT / PATCH / DELETE / HEAD / OPTIONS
2. **URL 输入** + 快速访问历史/集合
3. **Params 管理**：Query 参数键值对维护（URL 自动拼接）
4. **Headers 管理**：请求头键值对维护
5. **Body**：
   - 无 Body（GET/HEAD 默认）
   - `application/json`（带格式化、语法错误提醒）
   - `x-www-form-urlencoded`（键值对）
   - `text/plain`
   - 文件上传（multipart/form-data，文件选择）
6. **发送请求** + **响应展示**：
   - 状态码/耗时/响应头
   - 响应体：JSON 自动格式化高亮（`egui` TextEdit 只读显示 + 纯文本 fallback）、图片类型提示
   - Raw / 格式化 切换
7. **SSL 证书校验开关**（跳过自签名）
8. **重定向跟随开关**
9. **超时设置**

### 4.2 高级功能（用户确认需要）
1. **认证（Authorization）**：Basic、Bearer Token、自定义 Header（可选附加到请求头）
2. **代理设置**：系统代理 / 自定义 HTTP(S) 代理
3. **历史记录**：自动保存最近请求（含 method/url/headers/body），支持一键回放、按 URL 去重、可删除、搜索
4. **集合管理（Collection）**：分组保存请求（目录树），命名/保存/加载/删除，导出导入 JSON
5. **环境变量（Environment）**：定义全局 + 可命名环境（如 dev/prod）变量集；请求字段支持 `{{var}}` 占位符替换；切换环境即时生效

### 4.3 非目标（明确不做）
- WebSocket / gRPC / GraphQL 支持
- 自动代码生成、API 文档、协作/云同步
- 请求脚本/断言/测试自动化
- 拖拽式流程编排

---

## 五、项目结构（Proposed Changes）

```
d:\code\workspace\newcode\
├── Cargo.toml                 # 工程与依赖声明
├── build.rs                   # winres 嵌入图标/版本信息（应用图标资源）
├── assets/
│   └── app.ico                # 应用图标（可先用占位，后续替换）
├── resources/
│   └── app.exe.manifest       # Windows 高 DPI 清单（可选）
└── src/
    ├── main.rs                # 入口：初始化 app、窗口、挂载 UI
    ├── app.rs                 # AppState：全局 UI 状态、当前请求/响应、环境变量激活
    ├── ui/
    │   ├── mod.rs
    │   ├── request_tab.rs     # 请求编辑区（URL、方法、Params、Headers、Body）
    │   ├── response_panel.rs  # 响应展示区（状态、耗时、头、Body 格式化）
    │   ├── history_panel.rs   # 历史记录侧栏/列表
    │   ├── collection_panel.rs# 集合管理树 + 保存/加载
    │   └── env_panel.rs       # 环境变量管理
    ├── http/
    │   ├── mod.rs
    │   └── client.rs          # reqwest 客户端配置（TLS/代理/超时/重定向）
    ├── model/
    │   ├── mod.rs
    │   └── request.rs         # 请求/集合/环境变量/历史 的数据结构（serde）
    ├── storage/
    │   ├── mod.rs
    │   └── db.rs              # rusqlite 建表、CRUD、JSON 导出导入
    └── util/
        ├── mod.rs
        └── vars.rs            # {{var}} 占位符解析与替换
```

### 5.1 数据建模（SQLite 表）

```sql
-- 集合（目录树）
CREATE TABLE collections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id INTEGER DEFAULT 0,      -- 0=根，支持分组层级
    name TEXT NOT NULL,
    sort INTEGER DEFAULT 0
);
-- 集合内保存的请求
CREATE TABLE saved_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    collection_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    params TEXT NOT NULL DEFAULT '[]',   -- JSON
    headers TEXT NOT NULL DEFAULT '[]',  -- JSON
    body_type TEXT NOT NULL DEFAULT 'none',
    body TEXT DEFAULT '',
    auth TEXT DEFAULT '{}',              -- JSON: {type, username, password, token}
    sort INTEGER DEFAULT 0
);
-- 历史记录
CREATE TABLE history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    params TEXT NOT NULL DEFAULT '[]',
    headers TEXT NOT NULL DEFAULT '[]',
    body_type TEXT NOT NULL DEFAULT 'none',
    body TEXT DEFAULT '',
    auth TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
-- 环境变量（多个环境共享一表，通过 env_name 区分；global 为全局）
CREATE TABLE environments (
    env_name TEXT NOT NULL,       -- 'global' 或自定义名
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (env_name, key)
);
```

### 5.2 依赖清单（Cargo.toml 预估）

```toml
[package]
name = "http-request-tool"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = "0.29"              # egui 框架
egui = "0.29"
reqwest = { version = "0.12", features = ["json", "multipart", "blocking"], default-features = false, features = ["native-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
chrono = "0.4"              # 时间戳/耗时
anyhow = "1"
rfd = "0.15"                # 文件选择框（原生对话框）
url = "2"                   # URL 解析/拼接
shell-words / 内置解析      # (可选)

[build-dependencies]
winres = "0.1"              # 图标/版本资源嵌入

[profile.release]
opt-level = "s"             # 体积优化，贴近便携需求
lto = true
codegen-units = 1
strip = true
```

> 注：`reqwest` 关闭默认 `default-tls`，启用 `native-tls`+`rustls-tls-native-roots`，避免额外证书依赖；编译 MSVC 原生工具链已就绪。

---

## 六、实施步骤（Implementation Steps）

按顺序执行，每步可独立验证：

1. **初始化工程**：`cargo init` 建 `Cargo.toml`，填入依赖；创建 `assets/app.ico` 占位图标；`build.rs` 用 winres 嵌图标；`git init` 初始化仓库。
2. **数据层**：实现 `storage/db.rs` 建表 SQL 与 CRUD（集合、历史、环境变量、JSON 导出/导入）；`model/request.rs` 定义 struct；`util/vars.rs` 实现 `{{var}}` 替换。
3. **HTTP 层**：`http/client.rs` 封装 reqwest：方法/URL/Params/Headers/Body 组装、TLS 跳过开关、代理、超时、重定向开关，返回状态码/耗时/响应头/响应体。
4. **基础 UI（先跑通最小闭环）**：`main.rs` + `ui/request_tab.rs` + `ui/response_panel.rs`：能输入 URL、选方法、编辑 Headers、发请求、看响应（状态/耗时/格式化 JSON）。**此步为里程碑：界面可点、请求可发、响应可见。**
5. **认证与高级请求配置**：UI 加入 Authorization 区、SSL 开关、超时、代理、重定向开关，均接入 `http/client.rs`。
6. **历史记录**：发送成功/失败均自动入库；`history_panel.rs` 列表展示、搜索、一键回放、删除。
7. **环境变量**：`env_panel.rs` 增删改环境与键值；替换逻辑接到 URL/Params/Headers/Body，支持切换。
8. **集合管理**：`collection_panel.rs` 分组树、保存当前请求、加载、删除、JSON 导出/导入。
9. **打磨**：Body 文件上传（multipart）、响应图片展示、UI 布局优化、错误中文提示、Dark/Light 主题切换。
10. **打包验证**：`cargo build --release` 产出单 exe；冒烟测试（发请求、保存/回放、切环境）；确认 `exe` 可在无源码目录单独运行；记录体积与启动表现。

> 第 4 步完成即达可用 MVP，后续 5-10 为增强。若时间/复杂度受限，可评估在此分批交付。

---

## 七、假设与决策（Assumptions & Decisions）

1. **界面语言**：中文（符合用户习惯），代码注释跟随中文。
2. **持久化位置**：SQLite 数据库文件放**应用同目录** `http-tool.db`（便携理念：带着文件夹走即可），而非 `%APPDATA%`。UTF-8 编码。
3. **首次运行**：自动建库建表；无额外配置文件。
4. **UI 形态**：单窗口 + 左侧导航（历史/集合）+ 上部请求区 + 下方响应区（类似 Postman 布局）；图标需后续替换为正式图标（先用占位）。
5. **Rust PATH 处理**：开发阶段用绝对路径 `~/.cargo/bin/cargo` 编译；在最终交付说明里提示用户加 PATH，不擅自改系统环境变量。
6. **网络库 TLS**：`native-tls`（Windows Schannel），默认信任系统根证书；"跳过证书校验"开关用于自签名调试。
7. **代理**：默认"跟随系统代理"（reqwest 默认行为），提供"自定义代理"输入框覆盖。
8. **多线程**：请求在网络 goroutine（`std::thread`）中执行，通过 `egui::Context::request_repaint` 回调主线程更新 UI，保证不卡界面。
9. **版本控制**：目录在 `D:\code\workspace` 下，若用户要求提交，遵循用户规则用 "李元昊【提交内容】" 提交规范前缀。
10. **非目标**：不含 WebSocket/GraphQL/脚本断言/云同步（见 4.3）。

---

## 八、验证方案（Verification）

1. **编译期**：`cargo build --release` 零错误零警告通过。
2. **单元测试**：`util/vars.rs` 的 `{{var}}` 替换、`model` 的 JSON 序列化、`storage/db.rs` CRUD（用临时 DB）各写 `#[cfg(test)]` 测试。
3. **手动冒烟（GUI）**：
   - 对 `https://httpbin.org/get` 发 GET，确认状态 200、Params 拼接正确。
   - POST + JSON body 到 `https://httpbin.org/post`，确认回显 body。
   - 配置 Bearer/自定义 Header，确认服务端收到。
   - 请求一个自签名 https 服务（本机开 https 测试）验证"跳过证书校验"开关生效。
   - 保存到集合 → 切换环境变量 → 回放历史，确认占位符替换正确。
   - 关闭再启动应用，确认历史/集合/环境均持久化。
4. **便携验证**：将生成的 `exe`（+ `http-tool.db` 首次生成）复制到其他目录双击运行，无运行时依赖报错。

---

## 九、交付物

- `d:\code\workspace\newcode\` 下的完整 Rust 工程（可编译）
- 编译产物 `target\release\http-request-tool.exe`（单文件便携）
- 功能：基础请求 + 认证/代理/TLS + 历史 + 集合 + 环境变量，全部可用
