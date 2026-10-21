[English](README.md) · **简体中文**

# writeup-platform

一个自托管的 **CTF Writeup 提交平台**，为对接
[**ret2shell**](https://github.com/ret2shell/ret2shell) 平台而构建。选手使用
ret2shell 的临时验证码登录，浏览训练场与赛事，并上传自己的 Writeup（PDF /
docx）。赛事管理员可以查看、批量下载和管理提交。

后端：**Rust**（Axum + sqlx/PostgreSQL）。前端：**Vue 3**（Vite + Pinia）。

## 功能特性

- **四种界面语言** — 简体中文、繁体中文、英语和日语，自动识别浏览器语言并保存手动选择。
- **与 ret2shell 单点登录** —— 用临时验证码向 ret2shell HTTP API 换取登录态；
  会话为 HS256 JWT，与 ret2shell 使用相同的 `signing_key` 签名。
- **训练场 / 赛事区分** —— `host_type = 0`（训练场）按用户提交；`host_type = 1`
  （赛事）按队伍共享，并可按比赛的开始 / 归档时间限制提交。
- **文件校验** —— 通过文件头（magic bytes）校验上传内容（docx 会检查
  `[Content_Types].xml`）。上传大小有上限（默认 50 MiB）。
- **按赛事限定格式** —— 比赛管理员可在管理页勾选允许的提交格式。未单独设置的赛事
  默认仅接受 PDF；已提交的 DOCX 仍可在线阅读。
- **在线阅读** —— PDF 与 DOCX 均可在站内直接阅读（分别使用 PDF.js 和
  docx-preview），历史 DOCX 提交也无需下载即可查看。
- **重复检测** —— 记录每次上传的 SHA-256；同一比赛下其他队伍/用户提交相同文件
  会被拒绝。
- **重复提交 / upsert** —— 再次上传会覆盖原文件（临时文件写入 + 原子重命名，
  并清理旧文件）。
- **管理工具** —— 各比赛管理员可列出全部 Writeup、下载单个文件、打包下载
  ZIP，以及删除记录。
- **赛事置顶** —— 比赛管理员可置顶一场赛事；此后打开平台时选手会直接进入该
  赛事的提交页，该赛事的管理员则直接进入管理页（站内后续跳转不受影响）。
- **友好的文件名** —— 下载文件命名为 `{赛事}-{队伍或用户}.{扩展名}`，并对非
  ASCII 名称做 RFC 5987 编码。

## 仓库结构

```
.
├── Cargo.toml              # Rust workspace
├── crates/
│   ├── config/             # TOML 配置加载
│   ├── database/           # sqlx 连接池、迁移、writeup 查询
│   └── server/             # Axum HTTP 服务（路由、鉴权、存储）
├── frontend/               # Vue 3 + Vite 单页应用
├── config/
│   └── config.toml.example # 复制为 config.toml 并填写
├── docker-compose.yml
├── Dockerfile              # 多阶段：rust 服务 + vue 前端 + nginx
└── nginx.conf              # SPA 回退 + /api 反向代理
```

## 工作原理

```
浏览器 ──▶ nginx ──▶ /api ──▶ Rust 服务 ──▶ PostgreSQL（自有）      : writeup 元数据
                                    │
                                    ├──▶ PostgreSQL（ret2shell，只读）: 比赛、队伍、用户
                                    └──▶ ret2shell HTTP API          : 验证码 / 登录
```

服务连接 **两个**数据库：自有库（存放 writeup 元数据，启动时自动执行迁移）和 ret2shell 数据库（**只读**，用于查询比赛列表、解析队伍、判断比赛管理员）。

## 快速开始（Docker Compose）

1. 创建部署配置：

   ```sh
   cp config/config.toml.example config/config.toml
   ```

   填入 `[auth].signing_key`（必须与 ret2shell 一致）、两个数据库的连接信息，
   以及 ret2shell API 的 `base_url`。

2. 创建 `.env` 文件：

   ```sh
   POSTGRES_PASSWORD=change-me
   LISTEN_PORT=3000
   ```

3. 启动全部服务：

   ```sh
   docker compose up -d --build
   ```

   应用将运行在 `http://localhost:${LISTEN_PORT:-3000}`。

## 配置说明

`config/config.toml`（参见 `config/config.toml.example`）：

| 配置段         | 键              | 说明                                                     |
| -------------- | --------------- | -------------------------------------------------------- |
| `server`       | `host`、`port`  | HTTP 监听地址（默认 `0.0.0.0:8080`）                     |
| `auth`         | `signing_key`   | HS256 JWT 密钥 —— **无需与 ret2shell 一致**              |
| `database`     | `host`…         | 本平台自有的 PostgreSQL                                   |
| `r2s_database` | `host`…         | ret2shell 的 PostgreSQL（**只读**）：比赛/队伍/用户       |
| `r2s_api`      | `base_url`      | ret2shell HTTP API（验证码登录代理）                     |
| `storage`      | `path`          | 上传文件的存储目录                                        |
| `storage`      | `max_file_size` | 上传大小上限（字节，默认 50 MiB）                         |

## 本地开发

**后端**（需要可访问的 PostgreSQL 和 ret2shell 库）：

```sh
cp config/config.toml.example config/config.toml   # 按本地环境调整 host
CONFIG_PATH=./config/config.toml cargo run -p server
```


**前端：**

```sh
cd frontend
npm install
npm run dev
```

前端通过 `/api` 调用后端，因此开发时需把它指向运行中的后端（在 Vite 中为
`/api` 添加 `server.proxy` 配置），如有需要再设置 `VITE_R2S_BASE`。


## API 一览

| 方法     | 路径                                          | 鉴权       | 用途                           |
| -------- | --------------------------------------------- | ---------- | ------------------------------ |
| `POST`   | `/api/account/login`                          | 无         | 用验证码换取 JWT               |
| `GET`    | `/api/games`                                  | 可选       | 列出训练场 / 赛事              |
| `GET`    | `/api/games/{game_id}`                         | 可选       | 按 ID 获取赛事详情             |
| `GET`    | `/api/games/pinned`                           | 可选       | 获取被置顶的赛事               |
| `GET`    | `/api/writeup`                                | 需要       | 列出我的 Writeup               |
| `POST`   | `/api/writeup/{game_id}`                      | 需要       | 提交 / 覆盖 Writeup            |
| `GET`    | `/api/writeup/{game_id}`                      | 需要       | 获取我在某比赛的 Writeup       |
| `GET`    | `/api/writeup/{game_id}/file`                 | 需要       | 下载我的文件                   |
| `GET`    | `/api/admin/game/{game_id}/writeup`           | 比赛管理员 | 列出某比赛的全部 Writeup       |
| `GET`    | `/api/admin/game/{game_id}/writeup/download`  | 比赛管理员 | 打包下载全部（ZIP）            |
| `GET`    | `/api/admin/game/{game_id}/writeup/{writeup_id}/file`| 比赛管理员 | 下载单个 Writeup               |
| `DELETE` | `/api/admin/game/{game_id}/writeup/{writeup_id}`     | 比赛管理员 | 删除某个 Writeup               |
| `POST`   | `/api/admin/game/{game_id}/pin`               | 比赛管理员 | 置顶该赛事                     |
| `DELETE` | `/api/admin/game/{game_id}/pin`               | 比赛管理员 | 取消置顶该赛事                 |

鉴权使用 `Authorization: Bearer <jwt>` 请求头。有效 token 需带有权限位 `0`
（账号已激活）；比赛管理员操作还额外要求权限位 `5`，**且**用户 id 在该比赛的
`admins` 列表中。


## 安全说明

- 上传的文件名会被清洗，存储路径由服务端生成的 id 拼接，而非用户输入。

## 相关项目

- [ret2shell](https://github.com/ret2shell/ret2shell) —— 本服务对接的 CTF 平台，
  提供鉴权与比赛/队伍数据。

## 许可证

基于 **GNU Affero 通用公共许可证 v3.0** 授权 —— 见 [LICENSE](LICENSE)。由于
AGPL 对网络使用同样生效，任何将修改版作为服务运行的人都必须向其用户提供对应
源码。
