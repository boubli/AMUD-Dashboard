# Contributing to AMUD Dashboard

Thank you for your interest in contributing to AMUD Dashboard! This guide will help you set up your local development environment, understand how to interact with the database, and test metrics locally.

---

## 🛠️ Prerequisites

Before you begin, ensure you have the following installed:

1. **Rust Toolchain**: Install via [rustup.rs](https://rustup.rs/). We use the `2021` edition.
2. **SQLite3** (Optional): A client to view the database (e.g., [DB Browser for SQLite](https://sqlitebrowser.org/) or the `sqlite3` CLI). Note that the Rust binary bundles SQLite internally, so no system development libraries are strictly required for compilation.

---

## 🚀 Spinning up the Rust Environment

The AMUD repository is structured as a Cargo workspace with two main members:
- `amud-server`: The backend Axum web server and WebSocket dashboard host.
- `amud-agent`: The background telemetry daemon that polls host and hypervisor metrics.

### 1. Run the Backend Server

To compile and start the server, run:

```bash
cargo run -p amud-server
```

On its first run, the server will:
1. Create a `data/` directory (relative to your current working directory).
2. Initialize an empty SQLite database (`data/amud.db`).
3. Generate a random bootstrap password for the `admin` user and print it to the terminal. Save this password, as it is shown only once!

To keep the database and build artifacts out of the repository tree, set `DB_PATH` and optionally `CARGO_TARGET_DIR` to paths on your machine before running cargo.

### 2. Run the Telemetry Agent

The agent requires the same shared secret as the server to authenticate its telemetry stream.

**On Windows (TCP Loopback):**
By default on Windows, both the server and the agent communicate via TCP on `127.0.0.1:8050`.
```cmd
# Set the secret (must match the secret stored in settings table)
set AMUD_AGENT_SECRET=yoursecret
cargo run -p amud-agent
```

**On Linux/Unix (UNIX Domain Socket):**
By default on Unix, they communicate via a UDS bind mount located at `/opt/amud/run/amud.sock`.
```bash
export AMUD_AGENT_SECRET=yoursecret
cargo run -p amud-agent
```

You can customize the communication endpoints by setting `AMUD_SOCKET_PATH` (Unix) or `AMUD_TCP_ADDR` (Windows/TCP).

---

## 💾 Interacting with the Local Database

AMUD uses an embedded SQLite database managed via `rusqlite`.

- **Default Location**: The database is stored at `data/amud.db` relative to your execution folder.
- **Customizing the DB Path**: Set the `DB_PATH` environment variable:
  ```bash
  export DB_PATH=/path/to/custom/amud.db
  ```
- **Inspecting Database State**: You can open `data/amud.db` in any SQLite browser. To inspect schemas or run queries, use the command line:
  ```bash
  sqlite3 data/amud.db "SELECT * FROM apps;"
  ```

---

## Before you push

Prefer the local CI mirror (fmt, clippy, tests, docs build):

```bash
./scripts/ci-check.sh
```

On Windows PowerShell:

```powershell
.\scripts\ci-check.ps1
```

Or run the same checks manually:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib
```

GitHub Actions also runs **`cargo audit`**.

**SonarCloud (forever config):** `sonar-project.properties` excludes theme packs / theme-guards (intentional `!important` + decorative motion) and ignores those CSS rules on `style.css` / docs CSS modules. Do **not** hand-fix hundreds of theme `!important`s after each push — update the Sonar config instead if a new intentional path shows up.

---

## 📊 Testing Metrics Locally

If you run the agent on your local machine, it will capture your host's CPU, RAM, and disk metrics. However, it will skip Proxmox VE (`pve`) or Docker container telemetry if they are not running or configured on your development machine.
