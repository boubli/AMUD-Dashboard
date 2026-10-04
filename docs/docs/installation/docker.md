---
sidebar_position: 3
---

# Docker Deployment

Deploying AMUD in a Docker environment containerizes the entire dashboard and telemetry ecosystem. We publish pre-built **amd64** (`x86_64`) images on Docker Hub for instant setup on typical Intel/AMD hosts.

:::info ARM64 / Raspberry Pi / Ampere
The Docker image is **amd64 only**. On ARM64 hosts, use the [native Linux install](/docs/installation/linux) or `update-amud.sh` â€” GitHub Releases ship `amud-server-arm64` and `amud-agent-arm64` binaries.
:::

---

## 1. How the Docker Ecosystem Communicates

AMUD uses a multi-container architecture in Docker to enforce isolation and minimize host resource usage:

```
                  +----------------------------------------------+
                  |                  DOCKER HOST                 |
                  |                                              |
                  |     +----------------------------------+     |
                  |     |       /var/run/docker.sock       |     |
                  |     +-----------------+----------------+     |
                  |                       ^                      |
                  |                       | (Read-Only Mount)    |
                  |                       v                      |
                  |     +-----------------+----------------+     |
                  |     |        amud-agent container      |     |
                  |     | - Telemetry agent daemon         |     |
                  |     +-----------------+----------------+     |
                  |                       |                      |
                  |                       | Writes IPC Telemetry |
                  |                       v                      |
                  |     +-----------------+----------------+     |
                  |     |    Shared Volume: amud_run       |     |
                  |     |    (Socket: /var/run/amud/...)   |     |
                  |     +-----------------+----------------+     |
                  |                       ^                      |
                  |                       | Reads IPC Telemetry  |
                  |                       v                      |
                  |     +-----------------+----------------+     |
                  |     |      amud-dashboard container    |     |
                  |     | - Serving Web UI (Port 8000)     |     |
                  |     +----------------------------------+     |
                  +----------------------------------------------+
```

* **`amud-dashboard`**: The core application server. It runs the Web UI, stores state inside SQLite, and serves dashboards to users over HTTP.
* **`amud-agent`**: The telemetry helper. It mounts the host's Docker socket to discover containers and monitor their active state.
* **`amud_run` Volume**: A high-speed, in-memory or directory volume sharing a Unix Domain Socket between the two containers. This allows them to transfer rich metric payloads locally with sub-millisecond latency.

---

## 2. Recommended: Docker Compose Setup

Using Docker Compose is the standard method for running AMUD. Create a `docker-compose.yml` file in your preferred configuration directory (e.g. `/opt/amud/`):

```yaml title="docker-compose.yml"
version: '3.8'

services:
  app:
    image: tradmss/amud-dashboard:latest
    container_name: amud_app
    restart: always
    ports:
      - "8000:8000"
    environment:
      - PORT=8000
      - BIND_ADDR=0.0.0.0
      - DB_PATH=/app/data/amud.db
      - PUID=${PUID:-99}
      - PGID=${PGID:-100}
      - AMUD_SOCKET_MODE=666
      - AMUD_SOCKET_PATH=/var/run/amud/amud.sock
      - AMUD_AGENT_SECRET=${AMUD_AGENT_SECRET:?Set AMUD_AGENT_SECRET in .env}
      - AMUD_ENABLE_PROXMOX=false # Set to true if running on Proxmox
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
    volumes:
      - ./data:/app/data
      - amud_run:/var/run/amud

  agent:
    image: tradmss/amud-dashboard:latest
    container_name: amud_agent
    user: "0:0"
    entrypoint: ["/app/amud-agent"]
    restart: always
    environment:
      - AMUD_SOCKET_PATH=/var/run/amud/amud.sock
      - AMUD_AGENT_SECRET=${AMUD_AGENT_SECRET:?Set AMUD_AGENT_SECRET in .env}
      - AMUD_DOCKER=${AMUD_DOCKER:-1} # Auto-enabled when docker.sock is mounted; set 0 to disable
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
    volumes:
      - amud_run:/var/run/amud
      - /var/run/docker.sock:/var/run/docker.sock:ro

volumes:
  amud_run:
    name: amud_run
```

### Required secret

Create a `.env` file next to `docker-compose.yml` (or export the variable in your shell):

```bash
AMUD_AGENT_SECRET=change-me-to-a-long-random-string
```

Both `amud-dashboard` and `amud-agent` **must** use the same value. The server and agent refuse to start without it.

### App data permissions (v1.7.2+)

The dashboard image runs as **UID 99 / GID 100** by default (matches Unraid `nobody:users`). On generic Linux hosts, create and own the data folder before first boot:

```bash
mkdir -p data && chown 99:100 data
```

`AMUD_SOCKET_MODE=666` lets the root agent connect to the dashboard Unix socket when the server runs as a non-root user. The agent service uses `user: "0:0"` for Docker socket access.

Unraid permission errors: [Troubleshooting â€” `.amud-secrets-key` permission denied](https://boubli.github.io/AMUD-Dashboard/docs/troubleshooting#unraid-secrets-key-permission-denied). `su-exec: setgroups` loop: [Troubleshooting â€” setgroups](https://boubli.github.io/AMUD-Dashboard/docs/troubleshooting#unraid-su-exec-setgroups-loop).

### Deploying the Stack
To start the services in detached background mode:
```bash
docker compose up -d
```

To view real-time log aggregates for diagnostic purposes:
```bash
docker compose logs -f
```

### Running Without Docker Socket (Optional)

If you do not need Docker container status badges on your dashboard and prefer to run the telemetry agent without mounting the host's `/var/run/docker.sock`, you can utilize the optional `docker-compose.no-docker.yml` override:

```bash
# Start the stack with the no-docker override applied
docker compose -f docker-compose.yml -f docker-compose.no-docker.yml up -d
```

When using this file, keep `AMUD_DOCKER=0` (the default) on the agent.

---

## 3. Alternative: Docker CLI Run

If you prefer using pure CLI commands without creating files, establish the shared volume and start both containers sequentially:

```bash
# 1. Create a shared volume for the Unix IPC socket
docker volume create amud_run

# 2. Start the dashboard web server (with hardened security settings)
docker run -d \
  --name amud_app \
  -p 8000:8000 \
  -v $(pwd)/data:/app/data \
  -v amud_run:/var/run/amud \
  -e PORT=8000 \
  -e BIND_ADDR=0.0.0.0 \
  -e DB_PATH=/app/data/amud.db \
  -e AMUD_SOCKET_PATH=/var/run/amud/amud.sock \
  -e AMUD_AGENT_SECRET=change-me-to-a-long-random-string \
  -e AMUD_ENABLE_PROXMOX=false \
  --cap-drop ALL \
  --security-opt no-new-privileges:true \
  --restart always \
  tradmss/amud-dashboard:latest

# 3. Start the telemetry agent
docker run -d \
  --name amud_agent \
  -v amud_run:/var/run/amud \
  -v /var/run/docker.sock:/var/run/docker.sock:ro \
  -e AMUD_SOCKET_PATH=/var/run/amud/amud.sock \
  -e AMUD_AGENT_SECRET=change-me-to-a-long-random-string \
  -e AMUD_DOCKER=0 \
  --entrypoint "/app/amud-agent" \
  --cap-drop ALL \
  --security-opt no-new-privileges:true \
  --restart always \
  tradmss/amud-dashboard:latest
```

---

## 4. Configuration Reference

You can pass these environment variables to adjust container configurations:

| Variable | Target Container | Default | Description |
|---|---|---|---|
| `PORT` | `amud_app` | `8000` | Port on which the Axum web server listens. |
| `DB_PATH` | `amud_app` | `/app/data/amud.db` | Directory path pointing to the SQLite database file. |
| `PUID` | `amud_app` | `99` | User ID for dashboard writes to the data volume (Unraid default: `nobody`). |
| `PGID` | `amud_app` | `100` | Group ID for dashboard writes (Unraid default: `users`). |
| `AMUD_SOCKET_MODE` | `amud_app` | `660` | Octal Unix socket permissions; use `666` in Docker when dashboard runs as non-root. |
| `AMUD_SOCKET_PATH` | Both | `/var/run/amud/amud.sock` | File path pointing to the Unix socket for agent-server IPC. |
| `AMUD_TCP_ADDR` | Both | `127.0.0.1:8050` | TCP bind address/port used for agent-server IPC on Windows or non-Unix setups. |
| `AMUD_AGENT_SECRET` | Both | *(Required)* | Shared authentication secret between dashboard and agent. |
| `BIND_ADDR` | `amud_app` | `127.0.0.1` | Set to `0.0.0.0` inside Docker so the container accepts external connections. |
| `AMUD_SECURE_COOKIES` | `amud_app` | `0` | Set to `1` to restrict session cookies to HTTPS connections. |
| `AMUD_DOCKER` | `amud_agent` | auto | Enabled when `/var/run/docker.sock` is mounted. Set `0` to disable container monitoring and start/stop/restart controls. |
| `PVE_NODE` | `amud_agent` | hostname | Proxmox node name when it differs from the container/host hostname. |
| `PVE_API_TOKEN` | `amud_agent` | *(None)* | Proxmox API token (if using agent on a PVE host; not needed for Docker monitoring). |
| `AMUD_ENABLE_PROXMOX` | `amud_app` | `false` | Set to `true` if installing on a Proxmox LXC to show the Proxmox settings tab in the UI. |

:::info For Non-Proxmox Users (Other OS)
If you are deploying AMUD via Docker on a standard Linux distribution (like Ubuntu, Fedora, Arch Linux, or Alpine) instead of Proxmox VE, the Proxmox Integration tab is hidden by default. If you do want to enable it, you can pass `-e AMUD_ENABLE_PROXMOX=true` to your dashboard container. Your host's CPU and Memory will still display perfectly fine on the dashboard under the "System" widget!
:::

---

## 5. Security Hardening Recommendations

When running AMUD in production environments, implement these security practices:

### A. Docker Socket Trust Boundary
When the agent mounts `/var/run/docker.sock`, Docker monitoring and container controls (start/stop/restart, CPU/RAM on app cards) are **enabled automatically**. Set `AMUD_DOCKER=0` on the agent to disable them while keeping the socket mount. The `:ro` modifier protects the socket file from being modified through the bind mount, but Docker's HTTP API can still process lifecycle requests over that socket. Treat the agent as trusted, and use a Docker socket proxy if you need method-level filtering.

### B. User Permissions (Non-Root Running)
If your host enforces strict daemon security, configure the agent to run under the host's `docker` group ID so it does not require root privileges. 

1. Identify the GID of the `docker` group on your host system:
   ```bash
   getent group docker | cut -d: -f3
   ```
2. In the `agent` service definition inside `docker-compose.yml`, specify the appropriate user grouping:
   ```yaml
   user: "1000:998" # Replace 998 with your host's docker GID
   ```

### C. Restricting Port Exposure
If you use a reverse proxy (e.g. Nginx, Caddy, Traefik), prevent exposing port `8000` to the public internet. Bind the port strictly to the host loopback interface by modifying the `app` service ports entry:
```yaml
ports:
  - "127.0.0.1:8000:8000"
```

---

## 6. Accessing the Dashboard

Navigate to your server's IP address on port 8000:
```
http://<YOUR_SERVER_IP>:8000/
```

:::tip Default Credentials
- **Username**: `admin`
- **Password**: `admin` (or `password` depending on version setup)
:::

---

## 7. Upgrading

Pull the latest image and recreate the stack:

```bash
docker compose pull
docker compose up -d
```

After upgrading:

1. **Hard-refresh the browser** (`Ctrl+Shift+R`) or [clear the PWA cache](../troubleshooting#pwa--browser-cache-issues).
2. If you use HTTPS via a reverse proxy, set `AMUD_SECURE_COOKIES=1` on the dashboard container â€” see [Security](../security.md).
3. Confirm both `amud_app` and `amud_agent` containers are healthy: `docker compose ps`.

---

## Additional nodes (remote agents)

To monitor other Docker hosts while keeping a single AMUD server, run **agent-only** containers elsewhere with `AMUD_SERVER_ADDR`, a unique `AMUD_NODE_TAG`, and the same `AMUD_AGENT_SECRET`. Enable `AMUD_AGENT_TCP_LISTEN` on the server. See [Multi-node agents](/docs/multi-node/overview).
