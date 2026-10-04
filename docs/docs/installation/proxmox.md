---
sidebar_position: 1
---

# Proxmox VE Installation

AMUD features a professional **Autopilot Installer** specifically built for Proxmox Virtual Environment (VE). The installation script automatically provisions a lightweight, ultra-lean Debian 12 LXC container to host the AMUD Server, and deploys the compiled, native AMUD Telemetry Agent directly onto your Proxmox hypervisor host.

---

## 1. Architecture Overview

To understand the installation steps, it is helpful to visualize how AMUD achieves its ultra-low resource profile (~26MB RAM combined):

```
+---------------------------------------------------------------------------------+
|                                 PROXMOX HOST                                    |
|                                                                                 |
|  +-----------------------+                    +------------------------------+  |
|  |  Proxmox VE REST API  |                    |      amud-agent (Host)       |  |
|  |  (LXC Container List) |                    |  - Polls local system info   |  |
|  +-----------+-----------+                    |  - Communicates with PVE API |  |
|              ^                                +--------------+---------------+  |
|              | (localhost HTTPS:8006 API)                    |                  |
|              v                                               v                  |
|  +---------------------------+                +------------------------------+  |
|  |     pveproxy service      |                |      /opt/amud/run/          |  |
|  +---------------------------+                |        amud.sock             |  |
|                                               +--------------+---------------+  |
|                                                              ^                  |
|  +-----------------------------------------------------------+---------------+  |
|  |  AMUD LXC CONTAINER (amud-dashboard)                      | Bind Mount    |  |
|  |                                                           v                  |
|  |  +--------------------------------------------------------+-----------+  |  |
|  |  |  amud-server (Rust Server)                                         |  |  |
|  |  |  - Serves HTTP UI on Port 8000                                     |  |  |
|  |  |  - Streams metrics to Browser via WebSockets                       |  |  |
|  |  +--------------------------------------------------------------------+  |  |
|  +--------------------------------------------------------------------------+  |
+---------------------------------------------------------------------------------+
```

The system is decoupled into two lightweight Rust daemons:
1. **`amud-agent` (runs on host)**: A high-frequency telemetry collector. It reads host hardware statistics directly from `/proc` and `/sys` interfaces, and queries the local Proxmox REST API (via port 8006) to fetch container states. It writes this metrics data to a Unix Domain socket.
2. **`amud-server` (runs in LXC container)**: A web backend built with Tokio and Axum. It reads telemetry data from the shared Unix socket, manages configuration settings in an embedded SQLite database, and broadcasts updates to connected browsers using persistent WebSockets.

---

## 2. Quick Install Command

Run the following command in your Proxmox Host shell as the `root` user:

```bash
curl -sSL https://raw.githubusercontent.com/boubli/AMUD-Dashboard/main/setup-amud.sh | bash
```

### What the Script Installs and Configures
1. **Agent Setup**:
   - Downloads the pre-compiled native `amud-agent` binary and installs it to `/usr/local/bin/amud-agent`.
   - Creates a dedicated `systemd` service (`amud-agent.service`) configured to auto-restart.
   - Initializes `/opt/amud/run` for runtime IPC socket files.
2. **LXC Container Provisioning**:
   - Downloads the official Debian 12 template if it is not already present in your local storage.
   - Spins up an LXC named `amud-dashboard` using the next available cluster ID from Proxmox (1 CPU Core, 256MB RAM, 4GB Disk). The installer prints the assigned ID at the end.
   - Establishes a secure bind-mount mapping `/opt/amud/run` on the host to `/opt/amud/run` in the container.
3. **Server Deployment**:
   - Sets up `/opt/amud/data` inside the container for the SQLite database.
   - Installs the `amud-server` binary and extracts UI assets to `/opt/amud/ui`.
   - Registers and starts the `amud.service` daemon.

---

## 3. LXC Console Login & Password Setup

If you want to log directly into the LXC container's terminal (via the Proxmox Web Console or SSH), you need to set a password for the `root` user. By default, Debian LXC templates do not have a pre-configured root password.

You can set or reset the password directly from your **Proxmox Host shell** without needing the old password:

```bash
# Set the password for the root user of your container
pct set <CONTAINER_ID> -password

# Alternatively, enter a direct shell inside the container to configure it
pct enter <CONTAINER_ID>
```

---

## 4. Initial Dashboard Access

Once the installation completes, the script displays the container's IP address.

1. Open your browser and navigate to:
   ```
   http://<YOUR_LXC_IP>:8000/
   ```
2. Log in using the default administrator credentials:
   - **Username**: `admin`
   - **Password**: `password`

:::warning Change Default Password Immediately
Log in, navigate to **Settings â†’ Admin Profile**, and change the administrator password immediately to secure your installation.
:::

---

## 5. Proxmox API Token Configuration

To display live status badges (**RUNNING**, **STOPPED**) for other containers on your dashboard, the `amud-agent` needs a Proxmox API token. Without a token, the agent will report host metrics (CPU/RAM/Disk), but your app cards will remain stuck on **"CHECKING..."**.

AMUD is designed with security in mind. Rather than using administrative credentials, we recommend setting up a restricted PVE user. If you wish to control container power actions (Start, Stop, Restart) directly from the AMUD dashboard, you must grant the user power management privileges (`VM.PowerMgmt`) in addition to audit permissions.

### Option A: Direct CLI Provisioning (Recommended for Pros)

For an automated, reproducible setup, paste the following script block directly into your Proxmox host terminal. It creates a restricted group, a local user, assigns audit (and optionally power management) privileges, and generates the API token:

```bash
# 1. Create a custom permission role
# Grant "VM.PowerMgmt" only if you want to start/stop/restart containers from AMUD
pveum role add AMUDAgentRole -privs "VM.Audit Sys.Audit VM.PowerMgmt"

# 2. Create a restricted local system user group and user
pveum group add amud-group
pveum user add amud@pve -group amud-group -comment "Telemetry Agent User"

# 3. Associate our custom role with the group across the cluster path
pveum aclmod / -group amud-group -role AMUDAgentRole

# 4. Generate the API Token for the user WITH privilege separation disabled (required)
pveum user token add amud@pve amud-token --privsep 0
```

The terminal will print the credentials. Look for the line containing:
```
PVEAPIToken=amud@pve!amud-token=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
```
Copy this value.

---

### Option B: Proxmox Web UI Provisioning

If you prefer to configure permissions visually:

1. **Create the Role**:
   - Navigate to **Datacenter â†’ Permissions â†’ Roles** and click **Create**.
   - Name the role `AMUDAgentRole`.
   - Check the following permissions:
     - **`VM.Audit`** (to inspect LXC/VM state)
     - **`Sys.Audit`** (to inspect host node details)
     - **`VM.PowerMgmt`** (optional, check this if you want to control container states like Start/Stop/Restart from the dashboard)
   - Click **Create**.
2. **Create the User**:
   - Navigate to **Datacenter â†’ Permissions â†’ Users** and click **Add**.
   - Set User name to `amud` and Realm to `pve`. Click **Add**.
3. **Assign Access Control Permissions (ACL)**:
   - Navigate to **Datacenter â†’ Permissions** and click **Add â†’ API Path Permission**.
   - Set **Path** to `/` (cluster root).
   - Select User `amud@pve`.
   - Select Role `AMUDAgentRole`. Click **Add**.
4. **Generate the Token**:
   - Navigate to **Datacenter â†’ Permissions â†’ API Tokens** and click **Add**.
   - Select User `amud@pve` and set Token ID to `amud-token`.
   - **âš ï¸ CRITICAL: Uncheck "Privilege Separation"**.
   - Click **Add** and copy the token secret displayed on your screen.

:::danger Privilege Separation Notice
If **Privilege Separation** is checked, the token is treated as an isolated entity with zero inherited permissions. This will cause the Proxmox API to return empty lists, leaving your dashboard cards stuck on **"CHECKING..."**. Always ensure it is unchecked.
:::

:::info Container Power Actions (Optional)
To enable dashboard buttons for starting, stopping, and restarting containers on native Proxmox LXC nodes, the API token must be assigned a role with `VM.PowerMgmt` privileges. If you prefer to keep AMUD completely read-only, you can omit `VM.PowerMgmt`; container power actions will then fail with a permission error (shown in the audit logs), but telemetry and status badges will continue to function normally.
:::

---

## 6. Saving and Testing Token in AMUD

Once you have your API token, configure it using AMUD's built-in settings panel:

1. Log into your **AMUD Dashboard** as an administrator.
2. Click the **Settings** gear icon in the top right.
3. Select the **Proxmox** tab.
4. Paste the complete API token value into the input field:
   ```
   PVEAPIToken=amud@pve!amud-token=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
   ```
5. Click **Test Connection**. 
   - The server will immediately send a verification request through the IPC socket to the host agent.
   - The host agent attempts to contact the Proxmox REST API using the supplied token.
   - If successful, a green confirmation message is shown: **"Connection Tested Successfully! Proxmox is reachable."**
   - If it fails, the panel displays diagnostic details (e.g. invalid signature, permission denied, or port unreachable) to help you fix the issue.
6. Click **Save Settings**. The configuration is saved to the SQLite database, and the server pushes the token to the agent in real time. No services need to be restarted.

---

## 7. Advanced / Headless Configuration

For infrastructure-as-code deployments or headless scripts, you can set the token directly on the Proxmox host using Systemd environment variables:

1. Edit the agent service configuration:
   ```bash
   nano /etc/systemd/system/amud-agent.service
   ```
2. Define the token inside the `[Service]` block:
   ```ini title="/etc/systemd/system/amud-agent.service"
   [Service]
   Type=simple
   ExecStart=/usr/local/bin/amud-agent
   Restart=always
   RestartSec=5
   Environment=AMUD_SOCKET_PATH=/opt/amud/run/amud.sock
   Environment="PVE_API_TOKEN=PVEAPIToken=amud@pve!amud-token=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
   ```
3. Reload systemd and restart the agent:
   ```bash
   systemctl daemon-reload
   systemctl restart amud-agent
   ```

---

## 8. Troubleshooting Socket Mounts

If the dashboard displays `0%` metrics for the host CPU/RAM/Disk, the agent and server cannot communicate via the Unix socket file.

1. **Verify socket creation on host**:
   ```bash
   ls -la /opt/amud/run/amud.sock
   ```
   You should see a socket file owned by root (or your runtime user) with permissions `srwxrwxrwx`.
2. **Verify mount mapping inside LXC**:
   ```bash
   CT_ID=$(pct list | awk '$3 == "amud-dashboard" {print $1}' | head -n1)
   pct config "$CT_ID" | grep mp0
   ```
   It should return: `mp0: /opt/amud/run,mp=/opt/amud/run`. If missing, append it to `/etc/pve/lxc/<CT_ID>.conf` and reboot the container:
   ```bash
   echo "mp0: /opt/amud/run,mp=/opt/amud/run" >> "/etc/pve/lxc/${CT_ID}.conf"
   pct reboot "$CT_ID"
   ```

---

## 9. Upgrading AMUD

Run the updater on the **Proxmox host** as root:

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/boubli/AMUD-Dashboard/main/update-amud.sh)
```

After the upgrade completes:

1. **Hard-refresh the browser** (`Ctrl+Shift+R`) or [clear the PWA cache](../troubleshooting#pwa--browser-cache-issues) so new JavaScript and the service worker load.
2. **Verify agent IPC** if host metrics show `0%` â€” see [AMUD Agent Secret](../troubleshooting#amud-agent-secret--ipc-authentication).
3. Review [Security](../security.md) if you serve AMUD over HTTPS (`AMUD_SECURE_COOKIES=1`).

For a broken or partial update, see [Update or Release Recovery](../troubleshooting#update-or-release-recovery).

---

## Multiple Proxmox nodes

Install `amud-agent` on each PVE host with a unique `AMUD_NODE_TAG` and `AMUD_SERVER_ADDR` aimed at the single AMUD server. See [Multi-node agents](/docs/multi-node/overview).
