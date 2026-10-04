---
sidebar_position: 5
---

# Bare-Metal Linux Installation

Deploying AMUD directly on a bare-metal Linux host (Debian, Ubuntu, Fedora, Arch Linux, Rocky Linux, etc.) ensures the absolute lowest memory footprint and direct, unvirtualized access to hardware sensors. 

To achieve a production-grade deployment, this guide details how to install the binaries, set up a dedicated low-privilege system user, apply secure folder permissions, and configure hardened systemd background services.

---

## 1. Prerequisites

- A systemd-compatible Linux distribution.
- CLI utilities installed: `curl`, `wget`, `tar`.
- Administrative rights (`sudo` or `root` shell).

---

## 2. Low-Privilege Security Model (User & Directories)

Running application servers as `root` exposes your host system to unnecessary risks. We will create a restricted system user and group named `amud` with no login shell, dedicated strictly to executing the AMUD dashboard server.

### Step 1: Create the system user and group
```bash
# Create a system group
sudo groupadd --system amud

# Create a system user associated with the group, with no login shell
sudo useradd --system \
             -g amud \
             -s /sbin/nologin \
             -c "AMUD Daemon User" \
             amud
```

### Step 2: Establish the directory tree
AMUD uses `/opt/amud/` for its application files:
- `/opt/amud/run`: Unix Domain Socket for IPC.
- `/opt/amud/data`: SQLite database.
- `/opt/amud/ui`: Web UI HTML, JS, and CSS static templates.

```bash
# Create the directory structure
sudo mkdir -p /opt/amud/run /opt/amud/data /opt/amud/ui
```

### Step 3: Apply ownership and permissions
The `amud` user must own the database and runtime folders. The agent (which runs as `root` to poll host metrics) and the server (running as `amud`) will both communicate via `/opt/amud/run/amud.sock`. We use group ownership and permissions to ensure they can read and write to the socket.

```bash
# Set folder ownership
sudo chown -R amud:amud /opt/amud

# Set folder permissions (775 allows group-write for local socket creation)
sudo chmod 775 /opt/amud/run
sudo chmod 770 /opt/amud/data
```

### Step 4: Generate a shared IPC secret
AMUD requires a shared cryptographically secure secret (`AMUD_AGENT_SECRET`) to authenticate communication between the server and the agent daemon. Generate a random secret string:

```bash
openssl rand -base64 32 | tr -d '/+=' | head -c 43
```

Save the generated value; you will need to add it to the environment configuration of both services in Section 4.

---

## 3. Download Release Assets

We distribute pre-compiled `x86_64` and `arm64` binaries for every stable release. On ARM64, use `update-amud.sh` (it picks `amud-server-arm64` / `amud-agent-arm64` automatically) or download the `-arm64` assets manually.

:::note Docker vs native on ARM64
Docker Hub images are **amd64 only**. For ARM64 homelabs, prefer native install below rather than running an emulated amd64 container.
:::

Download the dashboard server, telemetry agent, and frontend UI templates:

```bash
# 1. Download and install the amud-server binary
wget https://github.com/boubli/AMUD-Dashboard/releases/latest/download/amud-server
chmod +x amud-server
sudo mv amud-server /usr/local/bin/

# 2. Download and install the amud-agent binary
wget https://github.com/boubli/AMUD-Dashboard/releases/latest/download/amud-agent
chmod +x amud-agent
sudo mv amud-agent /usr/local/bin/

# 3. Download and extract the static frontend UI assets
wget https://github.com/boubli/AMUD-Dashboard/releases/latest/download/ui.tar.gz
sudo tar -xzf ui.tar.gz -C /opt/amud/ui/
```

---

## 4. Hardened Systemd Service Configurations

To ensure AMUD automatically starts on boot, restarts if it crashes, and operates inside a secure sandbox, configure systemd unit files with modern isolation parameters.

### A. AMUD Server Service (Hardened & Non-Root)

Create `/etc/systemd/system/amud-server.service`:
```bash
sudo nano /etc/systemd/system/amud-server.service
```

Paste the following configuration. Note the sandboxing parameters under `[Service]`:

```ini title="/etc/systemd/system/amud-server.service"
[Unit]
Description=AMUD Dashboard Server
After=network.target

[Service]
Type=simple
User=amud
Group=amud
WorkingDirectory=/opt/amud
ExecStart=/usr/local/bin/amud-server
Restart=always
RestartSec=5

# Environment variables
Environment=PORT=8000
Environment=BIND_ADDR=127.0.0.1
Environment=DB_PATH=/opt/amud/data/amud.db
Environment=AMUD_SOCKET_PATH=/opt/amud/run/amud.sock
Environment=AMUD_AGENT_SECRET=your_generated_secret_here # Paste secret generated in Step 4
Environment=AMUD_ENABLE_PROXMOX=false # Set to true if running on Proxmox
UMask=0002

# Sandboxing and security hardening
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
NoNewPrivileges=yes
ReadWritePaths=/opt/amud/data /opt/amud/run
ReadOnlyPaths=/opt/amud/ui

[Install]
WantedBy=multi-user.target
```

### B. AMUD Agent Service (System Telemetry)

The agent runs as `root` to poll host hardware statistics (reading `/proc` and `/sys/class` interfaces) and container states directly.

Create `/etc/systemd/system/amud-agent.service`:
```bash
sudo nano /etc/systemd/system/amud-agent.service
```

Paste the following configuration:

```ini title="/etc/systemd/system/amud-agent.service"
[Unit]
Description=AMUD Host Telemetry Agent
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/amud-agent
Restart=always
RestartSec=5
Environment=AMUD_SOCKET_PATH=/opt/amud/run/amud.sock
Environment=AMUD_AGENT_SECRET=your_generated_secret_here # MUST match the server secret above
UMask=0002

[Install]
WantedBy=multi-user.target
```

---

## 5. Enable and Start Services

Reload systemd to detect the new configurations, enable the services to launch at boot, and start them:

```bash
# Reload systemd configuration
sudo systemctl daemon-reload

# Enable and start services immediately
sudo systemctl enable --now amud-server
sudo systemctl enable --now amud-agent
```

---

## 6. Verification and Diagnostics

### Service Status Checks
Verify that both services are active and running:
```bash
sudo systemctl status amud-server
sudo systemctl status amud-agent
```

### Unix Socket Verification
Confirm that the agent and server are sharing the IPC socket:
```bash
ls -la /opt/amud/run/
```
You should see the socket file:
```
srwxrwxr-x 1 amud amud 0 Jun  9 18:00 amud.sock
```

### Reading Daemon Logs
To inspect real-time log outputs or troubleshoot issues, use `journalctl`:

```bash
# View dashboard server logs
sudo journalctl -u amud-server -f -n 50

# View telemetry agent logs
sudo journalctl -u amud-agent -f -n 50
```

---

## 7. Accessing the Dashboard

Navigate to your server's IP address on port 8000:
```
http://<YOUR_SERVER_IP>:8000/
```

:::tip Default Credentials
- **Username**: `admin`
- **Password**: `password` (or `admin` depending on configuration)
:::

---

## 8. Upgrading

Download the latest release binaries and UI bundle, then replace `/usr/local/bin/amud-server`, `/usr/local/bin/amud-agent`, and `/opt/amud/ui` per [Download Release Assets](#3-download-release-assets). Restart both services:

```bash
sudo systemctl restart amud-server amud-agent
```

After upgrading:

1. **Hard-refresh the browser** (`Ctrl+Shift+R`) or [clear the PWA cache](../troubleshooting#pwa--browser-cache-issues).
2. Set `AMUD_SECURE_COOKIES=1` when serving over HTTPS â€” see [Security](../security.md).

---

## Additional hosts (agent only)

On secondary machines, install `amud-agent` with `AMUD_SERVER_ADDR`, `AMUD_NODE_TAG`, and the shared secret — no local `amud-server`. Full guide: [Multi-node agents](/docs/multi-node/overview).
