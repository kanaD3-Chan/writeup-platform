**English** · [简体中文](README-cn.md)

# writeup-platform

A self-hosted **CTF writeup submission platform**, built to plug into the
[**ret2shell**](https://github.com/ret2shell/ret2shell) CTF platform. Players log in with a ret2shell
captcha code, browse training arenas and competitions, and upload their writeups
(PDF / docx). Game admins can review, batch-download, and manage submissions.

Backend: **Rust** (Axum + sqlx/PostgreSQL). Frontend: **Vue 3** (Vite + Pinia).

## Features

- **Four UI languages** — Simplified Chinese, Traditional Chinese, English, and Japanese, with browser language detection and a saved language preference.
- **Single sign-on with ret2shell** — login by exchanging a temporary captcha
  code against the ret2shell HTTP API; sessions are HS256 JWTs signed with the
  same `signing_key` as ret2shell.
- **Training vs. competition** — `host_type = 0` (training) submissions are per
  user; `host_type = 1` (competition) submissions are shared per team and can be
  gated by the game's start / archive time.
- **File validation** — uploads are verified by magic bytes (docx is checked for
  `[Content_Types].xml`). Uploads are capped (default 50 MiB).
- **Per-game formats** — game admins choose which formats players may submit on
  the management page. Unless a game opts in, it accepts PDF only; existing DOCX
  submissions stay readable through the online reader.
- **Online reading** — PDF and DOCX writeups open in a built-in reader (PDF.js
  and docx-preview), so even historical DOCX submissions can be read without
  downloading them.
- **Duplicate detection** — SHA-256 of every upload is stored; the same file for
  the same game from another team/user is rejected.
- **Resubmission / upsert** — uploading again overwrites the previous file
  (atomic temp-file write + rename, old file cleaned up).
- **Admin tools** — per-game admins can list all writeups, download a single
  file, download everything as a ZIP, and delete records.
- **Pinned event** — game admins can pin one event; opening the platform then
  takes players to that event's submit page and its admins to its management
  page (later navigation through the site is unaffected).
- **Nice filenames** — downloads are named `{game}-{team-or-user}.{ext}` with
  RFC 5987 encoding for non-ASCII names.

## Repository layout

```
.
├── Cargo.toml              # Rust workspace
├── crates/
│   ├── config/             # TOML config loader
│   ├── database/           # sqlx pool, migrations, writeup queries
│   └── server/             # Axum HTTP server (routes, auth, storage)
├── frontend/               # Vue 3 + Vite SPA
├── config/
│   └── config.toml.example # copy to config.toml and fill in
├── docker-compose.yml
├── Dockerfile              # multi-stage: rust server + vue frontend + nginx
└── nginx.conf              # SPA fallback + /api reverse proxy
```

## How it works

```
Browser ──▶ nginx ──▶ /api ──▶ Rust server ──▶ PostgreSQL (own)      : writeup metadata
                                    │
                                    ├──▶ PostgreSQL (ret2shell, RO)  : games, teams, users
                                    └──▶ ret2shell HTTP API          : captcha / login
```

The server connects to **two** databases: its own (writeup metadata, migrations
run automatically on startup) and the ret2shell database **read-only** (used to
list games, resolve teams, and check game admins). 

## Quick start (Docker Compose)

1. Create the deployment config:

   ```sh
   cp config/config.toml.example config/config.toml
   ```

   Fill in `[auth].signing_key` (must match ret2shell), both database
   credentials, and the ret2shell API `base_url`.

2. Create a `.env` file:

   ```sh
   POSTGRES_PASSWORD=change-me
   LISTEN_PORT=3000
   ```

3. Start everything:

   ```sh
   docker compose up -d --build
   ```

   The app is served on `http://localhost:${LISTEN_PORT:-3000}`.

## Configuration reference

`config/config.toml` (see `config/config.toml.example`):

| Section        | Key              | Notes                                                    |
| -------------- | ---------------- | -------------------------------------------------------- |
| `server`       | `host`, `port`   | HTTP bind address (default `0.0.0.0:8080`)               |
| `auth`         | `signing_key`    | HS256 JWT secret — **no need match ret2shell**              |
| `database`     | `host`…          | This platform's own PostgreSQL                            |
| `r2s_database` | `host`…          | ret2shell PostgreSQL (**read-only**); games/teams/users  |
| `r2s_api`      | `base_url`       | ret2shell HTTP API (captcha login proxy)                 |
| `storage`      | `path`           | Where uploaded files are stored                          |
| `storage`      | `max_file_size`  | Max upload in bytes (default 50 MiB)                     |

## Local development

**Backend** (needs a reachable PostgreSQL and ret2shell DB):

```sh
cp config/config.toml.example config/config.toml   # adjust hosts to your local setup
CONFIG_PATH=./config/config.toml cargo run -p server
```


**Frontend:**

```sh
cd frontend
npm install
npm run dev
```

The SPA calls the API under `/api`, so in development point it at a running
backend (add a Vite `server.proxy` entry for `/api`) and set `VITE_R2S_BASE`
if needed.


## API overview

| Method   | Path                                          | Auth          | Purpose                        |
| -------- | --------------------------------------------- | ------------- | ------------------------------ |
| `POST`   | `/api/account/login`                          | none          | Exchange captcha code for JWT  |
| `GET`    | `/api/games`                                  | optional      | List training arenas / events  |
| `GET`    | `/api/games/{game_id}`                         | optional      | Get a game by ID               |
| `GET`    | `/api/games/pinned`                           | optional      | Get the pinned game            |
| `GET`    | `/api/writeup`                                | required      | List my writeups               |
| `POST`   | `/api/writeup/{game_id}`                      | required      | Submit / replace a writeup     |
| `GET`    | `/api/writeup/{game_id}`                      | required      | Get my writeup for a game      |
| `GET`    | `/api/writeup/{game_id}/file`                 | required      | Download my file               |
| `GET`    | `/api/admin/game/{game_id}/writeup`           | game admin    | List all writeups for a game   |
| `GET`    | `/api/admin/game/{game_id}/writeup/download`  | game admin    | Download all as ZIP            |
| `GET`    | `/api/admin/game/{game_id}/writeup/{writeup_id}/file`| game admin    | Download one writeup           |
| `DELETE` | `/api/admin/game/{game_id}/writeup/{writeup_id}`     | game admin    | Delete a writeup               |
| `POST`   | `/api/admin/game/{game_id}/pin`               | game admin    | Pin the game                   |
| `DELETE` | `/api/admin/game/{game_id}/pin`               | game admin    | Unpin the game                 |

Authentication uses a `Authorization: Bearer <jwt>` header. A valid token must
carry permission `0` (activated account); game-admin operations additionally
require permission `5` **and** the user's id in the game's `admins` list.


## Security notes

- Uploaded filenames are sanitized and storage paths are constructed from
  server-generated ids, not user input.

## Related projects

- [ret2shell](https://github.com/ret2shell/ret2shell) — the CTF platform this
  service integrates with for authentication and game/team data.

## License

Licensed under the **GNU Affero General Public License v3.0** — see
[LICENSE](LICENSE). Because the AGPL applies to network use, anyone running a
modified version as a service must offer the corresponding source to its users.
