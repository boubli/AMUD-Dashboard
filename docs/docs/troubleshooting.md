---
sidebar_position: 4
---

# Troubleshooting

This guide covers the most common issues users encounter when deploying AMUD on Proxmox VE, and how to diagnose and resolve them quickly.

---

## Find Your AMUD Container ID

The autopilot installer creates an LXC named `amud-dashboard`, but the numeric ID depends on your cluster (it is **not always `101`**). Before running any `pct exec` command below, look up your container ID on the Proxmox host:

```bash
pct list | awk '$3 == "amud-dashboard" {print $1}' | head -n1
```

Or list all containers and pick the ID for `amud-dashboard`:

```bash
pct list
```

In the examples below, replace `<CT_ID>` with that number (for example `102`, `111`, etc.).

---

## Apps Stuck on "CHECKING..."

**Symptom:** Your dashboard loads correctly and shows live CPU, RAM, and Disk metrics in the top bar, but individual application cards display a grey **"CHECKING..."** badge instead of **RUNNING** or **STOPPED**.

**After a full Proxmox host reboot:** host metrics usually appear first. LXC badges stay on **CHECKING...** (or last-known status from **v1.8.12+**) until the Proxmox API and containers are ready — that is normal for a short window. If badges remain stuck for several minutes after the node is fully up, continue with the steps below.

**Root Cause:** The AMUD agent cannot retrieve the list of LXC containers from the Proxmox API. This is almost always caused by a missing or misconfigured API token.

### Diagnostic Steps

**Step 1 — Check the agent logs:**

```bash
journalctl -u amud-agent --no-pager -n 20
```

Look for lines starting with `[LXC]`. The log output will tell you exactly what's wrong:

| Log Message | Meaning | Fix |
|---|---|---|
| `PVE_API_TOKEN not set or empty` | The agent cannot find the token | [Set the API token →](#fix-1--set-the-api-token) |
| `Successfully fetched 0 containers` | Token works but has no permissions | [Disable Privilege Separation →](#fix-2--disable-privilege-separation) |
| `PVE API returned HTTP 401` | Token secret is invalid or expired | [Recreate the token →](#fix-3--recreate-the-token) |
| `PVE API returned HTTP 500/595` | Wrong Proxmox node name | [Check hostname →](#fix-4--check-node-hostname) |
| `HTTP request to PVE API failed` | Cannot reach Proxmox API on port 8006 | [Check firewall →](#fix-5--check-network-connectivity) |
| No `[LXC]` lines at all | Agent binary is outdated | [Update the agent →](#fix-6--update-the-agent) |

---

### Fix 1 — Set the API Token

If you see `PVE_API_TOKEN not set or empty`, the agent's systemd service file doesn't have the token configured.

1. Create an API token in Proxmox (see [Proxmox Installation → API Token Setup](/docs/installation/proxmox#5-proxmox-api-token-configuration)).
2. Edit the agent service file:

```bash
nano /etc/systemd/system/amud-agent.service
```

3. Add this line under `[Service]` (replace with your actual token):

```ini
Environment="PVE_API_TOKEN=PVEAPIToken=root@pam!amud=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
```

4. Restart:

```bash
systemctl daemon-reload
systemctl restart amud-agent
```

---

### Fix 2 — Disable Privilege Separation

If you see `Successfully fetched 0 containers from PVE`, your token is valid but **Privilege Separation** is blocking it from reading any data.

:::danger This is the #1 most common issue
Proxmox enables **Privilege Separation** by default when creating API tokens. When enabled, the token has **zero permissions** — even if it belongs to `root@pam`. The API will authenticate your request successfully (HTTP 200) but return an empty container list.
:::

**To fix this:**

1. Go to **Proxmox Web UI → Datacenter → Permissions → API Tokens**.
2. **Delete** the existing token.
3. Click **Add** to create a new one:
   - User: `root@pam`
   - Token ID: `amud`
   - **⚠️ UNCHECK "Privilege Separation"**
4. Copy the new secret.
5. Update the agent service file with the new token:

```bash
nano /etc/systemd/system/amud-agent.service
```

6. Replace the old `PVE_API_TOKEN` line and restart:

```bash
systemctl daemon-reload
systemctl restart amud-agent
```

7. Verify:

```bash
journalctl -u amud-agent --no-pager -n 10
```

You should now see: `Successfully fetched XX containers from PVE.`

---

### Fix 3 — Recreate the Token

If you see `PVE API returned HTTP 401`, the token secret is invalid. This can happen if:
- You copied the secret incorrectly (missing characters)
- The token was deleted or regenerated in Proxmox
- There is a typo in the service file

**To fix this:**

1. Go to **Proxmox → Datacenter → Permissions → API Tokens**.
2. Delete the old token and create a new one (with **Privilege Separation unchecked**).
3. Carefully copy the entire secret value.
4. Update the service file, making sure:
   - The format is exactly: `PVEAPIToken=user@realm!tokenid=secret`
   - The line has both opening **and closing** double quotes
   - There are no trailing spaces or newlines

```ini
# ✅ Correct
Environment="PVE_API_TOKEN=PVEAPIToken=root@pam!amud=4af82325-36a8-4e24-ab33-0fd71276e31b"

# ❌ Wrong — missing closing quote
Environment="PVE_API_TOKEN=PVEAPIToken=root@pam!amud=4af82325-36a8-4e24-ab33-0fd71276e31b

# ❌ Wrong — missing PVEAPIToken= prefix
Environment="PVE_API_TOKEN=root@pam!amud=4af82325-36a8-4e24-ab33-0fd71276e31b"
```

---

### Fix 4 — Check Node Hostname

If you see `PVE API returned HTTP 500` or `HTTP 595`, the agent may be using the wrong Proxmox node name.

The agent automatically reads the hostname from `/etc/hostname` on the Proxmox host. You can verify it matches your actual node name:

```bash
# Check what the agent will use
cat /etc/hostname

# Compare with your actual Proxmox node name
pvesh get /nodes --output-format json | grep node
```

If they don't match, update `/etc/hostname` to match your Proxmox node name and restart the agent.

---

### Fix 5 — Check Network Connectivity

If you see `HTTP request to PVE API failed`, the agent cannot reach the Proxmox API on `https://localhost:8006`.

Verify the Proxmox API is accessible locally:

```bash
curl -k https://localhost:8006/api2/json/version
```

If this fails, check:
- Is the `pveproxy` service running? (`systemctl status pveproxy`)
- Is a firewall blocking port 8006 on localhost? (`iptables -L -n`)

---

### Fix 6 — Update the Agent

If you see no `[LXC]` log lines at all, your agent binary is outdated and doesn't include the diagnostic logging or the dynamic hostname fix.

Update to the latest version:

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/boubli/AMUD-Dashboard/main/update-amud.sh)
```

---

## App Name Matching

Even when LXC data is flowing correctly, an app card may still show "CHECKING..." if the **Application Name** in the AMUD dashboard doesn't match the **LXC container name** in Proxmox.

AMUD uses fuzzy matching — the app name and LXC name only need to **partially overlap** (case-insensitive). For example:

| Dashboard App Name | Proxmox LXC Name | Match? |
|---|---|---|
| `jellyfin` | `jellyfin` | ✅ Exact match |
| `Nginx Proxy Manager` | `nginx-proxy-manager` | ❌ No match (hyphens vs spaces) |
| `qbittorrent` | `qbittorrent` | ✅ Exact match |
| `immich` | `immich-server` | ✅ Partial match (`immich` is inside `immich-server`) |
| `My Plex Server` | `plex` | ✅ Partial match (`plex` is inside `my plex server`) |

**Best practice:** Name your apps in the dashboard to match (or contain) the exact LXC container name shown in Proxmox.

---

## Dashboard Refuses Connection After Update

**Symptom:** After running `update-amud.sh`, the updater reports success, but opening `http://<LXC_IP>:8000` shows **ERR_CONNECTION_REFUSED** or **This site can't be reached**.

**Most likely cause:** The AMUD server is running inside the LXC, but it is listening only on `127.0.0.1:8000` inside the container instead of the LXC network interface. This can happen after upgrading to a release where the secure default bind address changed to `127.0.0.1`.

### Diagnostic Steps

Run these commands from the Proxmox host:

```bash
pct exec <CT_ID> -- systemctl status amud
pct exec <CT_ID> -- ss -tlnp | grep 8000
```

If the `ss` output shows `127.0.0.1:8000`, the dashboard is only reachable from inside the LXC. Set the LXC service bind address to all interfaces:

```bash
pct exec <CT_ID> -- bash -c "grep -q '^Environment=BIND_ADDR=' /etc/systemd/system/amud.service && sed -i 's|^Environment=BIND_ADDR=.*|Environment=BIND_ADDR=0.0.0.0|' /etc/systemd/system/amud.service || sed -i '/^\[Service\]/a Environment=BIND_ADDR=0.0.0.0' /etc/systemd/system/amud.service"
pct exec <CT_ID> -- systemctl daemon-reload
pct exec <CT_ID> -- systemctl restart amud
pct exec <CT_ID> -- ss -tlnp | grep 8000
```

The final command should show `0.0.0.0:8000`. Then open:

```text
http://<LXC_IP>:8000
```

If nothing is listening on port `8000`, check the server logs instead:

```bash
pct exec <CT_ID> -- journalctl -u amud -n 80 --no-pager
```

:::info Fixed in newer updater scripts
Recent versions of `setup-amud.sh` and `update-amud.sh` set `BIND_ADDR=0.0.0.0` automatically for Proxmox LXC deployments.
:::

---

## Agent Keeps Disconnecting (Broken Pipe)

**Symptom:** You see repeated `Broken pipe (os error 32)` errors in the agent logs.

**Cause:** This happens when the AMUD server inside the LXC container restarts (e.g., during an update) and the Unix socket connection is interrupted.

**This is normal behavior.** The agent will automatically reconnect within 5 seconds. If it keeps happening continuously without recovery:

1. Check the server is running inside the LXC:

```bash
pct exec <CT_ID> -- systemctl status amud
```

2. Check the socket file exists and is writable:

```bash
ls -la /opt/amud/run/amud.sock
```

3. Restart both services:

```bash
pct exec <CT_ID> -- systemctl restart amud
systemctl restart amud-agent
```

---

## Dashboard Loads but Shows No Metrics

**Symptom:** The dashboard loads, you can log in, but the CPU/RAM/Disk bars are all at 0% and no data appears.

**Cause:** The agent is not connected to the server.

1. Check the agent status:

```bash
systemctl status amud-agent
```

2. Check the socket bind-mount is configured:

```bash
grep mp0 /etc/pve/lxc/<CT_ID>.conf
```

You should see: `mp0: /opt/amud/run,mp=/opt/amud/run`

3. If missing, add it manually and restart:

```bash
echo "mp0: /opt/amud/run,mp=/opt/amud/run" >> /etc/pve/lxc/<CT_ID>.conf
pct reboot <CT_ID>
systemctl restart amud-agent
```

### Fix 3 — Unprivileged LXC Socket Directory Permission (Proxmox)

**Symptom:** In the `amud-agent` logs on the host, you see:
```text
Failed to connect to dashboard daemon: Connection refused (os error 111)
```

**Cause:** By default, unprivileged LXC containers map the guest's `root` user to UID `100000` on the Proxmox host. If the host socket folder `/opt/amud/run` is owned by `root:root` (UID `0`) with `770` permissions, the containerized `amud-server` cannot create or bind to the `/opt/amud/run/amud.sock` socket file.

**Fix:** Change the host directory ownership to the unprivileged container root user's mapped UID:

```bash
chown -R 100000:100000 /opt/amud/run
pct exec <CT_ID> -- systemctl restart amud
systemctl restart amud-agent
```

---

## Unraid: `.amud-secrets-key` permission denied {#unraid-secrets-key-permission-denied}

**Symptom:** Dashboard container exits on first boot with:

```
Failed to initialize AMUD secrets encryption key: "write secrets key file /app/data/.amud-secrets-key: Permission denied (os error 13)"
```

The **AMUD-Agent** container may still appear running — it does not create this file.

**Cause:** Unraid Community Applications creates appdata as `nobody:users` (UID **99**). On images before **v1.7.2**, the dashboard ran as hardened root (`--cap-drop=ALL`) and could not write to that folder. SQLite (`amud.db`) would fail next for the same reason.

**Fixed in v1.7.2+:** The Docker image runs the dashboard as **PUID 99 / PGID 100** (Unraid defaults). Recreate the **AMUD-Dashboard** container after updating.

**Still failing after v1.7.2?** See [Permission errors on appdata](./installation/unraid.md#permission-errors-on-appdata) below, or run:

```bash
ls -la /mnt/user/appdata/amud-dashboard/
```

**Note:** Setting `AMUD_SECRETS_KEY` alone does **not** fix this — the database file must also be writable in the same folder.

---

## Add App: Integration dropdown shows only None

**Symptom:** In **Add App** or **Edit App**, the **Integration** dropdown only lists **None** — no Radarr, Sonarr, Plex, etc.

**Cause:** On **v1.7.0–v1.7.2**, the script that loads `/api/integrations/manifest` was missing a CSP nonce. The browser blocked it silently under the dashboard's Content Security Policy.

**Fix:** Update to **v1.7.3+** and hard refresh (Ctrl+Shift+R). After update, open DevTools → **Network** — you should see `/api/integrations/manifest` return **200** with a `groups` array.

**Still empty after v1.7.3?** Confirm you are logged in as admin (manifest requires a session). Check **Console** for CSP or network errors.

---

## Unraid: `su-exec: setgroups` loop

**Symptom:** After updating to v1.7.2, dashboard logs repeat:

```
su-exec: setgroups(100): Operation not permitted
```

**Cause:** Early v1.7.2 images dropped to PUID 99 with `su-exec` at runtime. The Unraid template also sets `--cap-drop=ALL`, which blocks `setgroups()` — the container restarts in a loop. **Privileged mode is not required** and is not recommended.

**Fix:** Update to **v1.7.3+** (or latest `tradmss/amud-dashboard:latest`), **Force Update**, and **recreate** the dashboard container. Turn **Privileged** off if you enabled it as a workaround.

**Stuck on an older image?** Add `--user 99:100` to the dashboard **Extra Parameters** (before `--cap-drop=ALL`) as a temporary workaround, or update the image.

See [Permission errors on appdata](./installation/unraid.md#permission-errors-on-appdata).

---

## Unraid: permission denied on appdata

**Symptom:** **AMUD-Dashboard** or **AMUD-Agent** will not start, or logs show errors writing to `/data` or binding the socket in `run/`.

**Cause:** Host folders under `/mnt/user/appdata/amud-dashboard/` are not owned by the user the container runs as.

**Fix:** See the full Unraid guide — [Permission errors on appdata](./installation/unraid.md#permission-errors-on-appdata). **v1.7.2+** runs the dashboard as PUID 99 (Unraid default). Older images: `chown -R 0:0` or `chown -R 99:100` on both `data` and `run`. See also [`.amud-secrets-key` permission denied](#unraid-secrets-key-permission-denied).

After fixing ownership, restart dashboard then agent.

---

## Docker / Portainer: Permission Denied on docker.sock

**Symptom:** The agent log reports errors like:
```
[Docker] Error connecting to docker socket: Permission denied (os error 13)
```

**Cause:** The user executing the agent inside the container does not have permission to read/write the mapped `/var/run/docker.sock` on the host system.

**Fixes:**

1. **Run container as root (Recommended):**
   In your `docker-compose.yml`, make sure the `amud-agent` container runs as root. You can do this by omitting user specifications, as default is root, which has access to the socket.
2. **Change host socket permissions:**
   Alternatively, grant read/write access to the docker socket on the host machine:
   ```bash
   sudo chmod 666 /var/run/docker.sock
   ```

---

## Reverse Proxy: WebSockets Disconnect (0% Metrics)

**Symptom:** The dashboard Web UI loads fine, but all host metrics (CPU/RAM/Disk) remain at `0%` and no live statuses stream. In your browser console (F12), you see errors like:
```
WebSocket connection to 'wss://amud.yourdomain.com/ws' failed: Error during WebSocket handshake: Unexpected response code: 400
```

**Cause:** The reverse proxy (Nginx, NPM, Apache) is forwarding HTTP traffic but stripping the headers required to "upgrade" the connection to WebSockets.

**Fixes:**

1. **Verify WebSocket Headers:** Refer to the [Reverse Proxy Configuration](/docs/installation/reverse-proxy) guide and ensure the following headers are set in your proxy block:
   ```nginx
   proxy_http_version 1.1;
   proxy_set_header Upgrade $http_upgrade;
   proxy_set_header Connection "upgrade";
   ```
2. **Nginx Proxy Manager:** Edit your proxy host in the NPM interface, check the **Websockets Support** toggle box, and click **Save**.
3. **Cloudflare Tunnels:** Ensure WebSockets are enabled under your domain's **Network** settings in the Cloudflare dashboard.

---

## Database is Locked or Permission Denied

**Symptom:** You receive errors like `database is locked` or `ReadOnly / Permission Denied` when trying to save settings or add apps in the dashboard UI.

**Cause:**
- Multiple instances of `amud-server` are running and contesting access to the SQLite database.
- The user running the `amud-server` service does not have write access to `/opt/amud/data/` or `/opt/amud/data/amud.db`.

**Fixes:**

1. **Check for duplicate server processes:**
   ```bash
   ps aux | grep amud-server
   ```
   If multiple processes are running, stop the service and terminate all duplicates:
   ```bash
   sudo systemctl stop amud-server
   sudo killall amud-server
   sudo systemctl start amud-server
   ```
2. **Fix file permissions:**
   Ensure the directory and database file are writable:
   ```bash
   sudo chmod -R 777 /opt/amud/data
   ```

---

## Reset or Change the Admin Password from CLI

Use this when you cannot log in to the AMUD UI after an upgrade, browser session reset, or credential mistake.

### See Existing Users

Run on the Proxmox host:

```bash
pct exec <CT_ID> -- sqlite3 /opt/amud/data/amud.db "SELECT id, username, role FROM users;"
```

If `sqlite3` is missing inside the LXC:

```bash
pct exec <CT_ID> -- apt-get update
pct exec <CT_ID> -- apt-get install -y sqlite3
```

### Retrieve Initial Bootstrap Password

If this is a fresh installation and you did not save the random bootstrap password printed by the installer, you can retrieve it directly from the systemd journal logs. 

Run this command on your **Proxmox Host**:

```bash
pct exec <CT_ID> -- journalctl -u amud --no-pager | grep 'Password:'
```

It will print the seeded admin password, for example:
```text
amud-server[123]:    Password: xxxxx-xxxxx-xxxxx
```

### Reset `admin` Password to `admin`

```bash
pct exec <CT_ID> -- sqlite3 /opt/amud/data/amud.db "INSERT INTO users (username,password_hash,role) VALUES ('admin','8c6976e5b5410415bde908bd4dee15dfb167a9c873fc4bb8a81f6f2ab448a918','Admin') ON CONFLICT(username) DO UPDATE SET password_hash=excluded.password_hash, role='Admin';"
pct exec <CT_ID> -- systemctl restart amud
```

Then sign in with:

```text
username: admin
password: admin
```

Change the password immediately in **Settings → Security**.

### Change Any User Password from CLI

AMUD stores new passwords as Argon2id hashes. For emergency CLI recovery, you can still write a legacy SHA-256 hash; AMUD accepts it once and transparently upgrades it to Argon2id after the next successful login.

```bash
NEW_PASSWORD='your-new-password'
HASH=$(printf '%s' "$NEW_PASSWORD" | sha256sum | awk '{print $1}')
pct exec <CT_ID> -- sqlite3 /opt/amud/data/amud.db "UPDATE users SET password_hash='$HASH' WHERE username='admin';"
pct exec <CT_ID> -- systemctl restart amud
```

After signing in, use **Settings → Security** to set the password again so a fresh Argon2id hash is stored immediately.

---

## AMUD Agent Secret / IPC Authentication

`AMUD_AGENT_SECRET` is not your Proxmox API token. It is an internal shared secret between the dashboard server inside the LXC and the host telemetry agent.

### Check Both Sides Match

```bash
grep AMUD_AGENT_SECRET /etc/systemd/system/amud-agent.service
pct exec <CT_ID> -- grep AMUD_AGENT_SECRET /etc/systemd/system/amud.service
systemctl show amud-agent -p Environment
```

If the values differ, copy the container value to the host service:

```bash
SECRET=$(pct exec <CT_ID> -- grep AMUD_AGENT_SECRET /etc/systemd/system/amud.service | cut -d= -f2)
sed -i "s|^Environment=AMUD_AGENT_SECRET=.*|Environment=AMUD_AGENT_SECRET=${SECRET}|" /etc/systemd/system/amud-agent.service
systemctl daemon-reload
systemctl restart amud-agent
pct exec <CT_ID> -- systemctl restart amud
```

### Verify Recent Logs Only

Old rejected connections can remain in `journalctl`. Check logs after restarting:

```bash
journalctl -u amud-agent --no-pager --since "2 minutes ago"
pct exec <CT_ID> -- journalctl -u amud --no-pager --since "2 minutes ago"
```

---

## Update or Release Recovery

If an update stops halfway through, restart both services first:

```bash
pct exec <CT_ID> -- systemctl restart amud
systemctl restart amud-agent
```

Then rerun the fixed updater from `main`:

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/boubli/AMUD-Dashboard/main/update-amud.sh)
```

Check release assets before publishing a new GitHub release:

```bash
gh release view vX.Y.Z --repo boubli/AMUD-Dashboard
gh release download vX.Y.Z --repo boubli/AMUD-Dashboard --pattern SHA256SUMS --dir /tmp/amud-release-check
```

Required assets:

- `amud-server`
- `amud-agent`
- `ui.tar.gz`
- `SHA256SUMS`
- `setup-amud.sh`
- `update-amud.sh`
- `uninstall-amud.sh`

---

## App card body empty but ONLINE shows

**Symptom:** App name and **ONLINE** / **RUNNING** badge render, but the card body is empty — no CPU/RAM row, no Radarr queue, no integration stats. Often reported on **Unraid Docker** after an upgrade ([#15](https://github.com/boubli/AMUD-Dashboard/issues/15)).

**What still works:** URL health checks (the status badge). That does **not** prove container metrics or integrations are loading.

### Quick checks

1. **Hard refresh** — `Ctrl+Shift+R` (see [PWA / Browser Cache](#pwa--browser-cache-issues) below).
2. **WebSocket pill** (top bar) — should say **Live**. If **Offline** / **Reconnecting**, fix `/ws` first ([Reverse Proxy WebSockets](#reverse-proxy-websockets-disconnect-0-metrics)).
3. **Edit one app** — confirm **Show container metrics** is enabled if you expect CPU/RAM.
4. **Integration apps** (Radarr, Prowlarr, etc.) — need a valid API URL + key; open browser DevTools → Network and check `/api/apps/<id>/integration` is not 401/500.
5. **View page source** on the dashboard — search for `data-lxc-metrics` or `app-card-metrics-slot` inside the card. If missing, metrics are disabled in app settings, not a CSS issue.

### Unraid Docker

- **AMUD-Agent** should use **host** network and bind-mount disk paths when using host telemetry mapping ([Unraid Step 3b](./installation/unraid.md#step-3b--host-network-and-disk-bind-mounts-v1557)).
- Agent needs Docker socket access for per-container CPU/RAM on cards.

### Simulate Unraid on Proxmox (bridge Docker)

If you only have Proxmox, you can approximate an Unraid-style stack:

1. Run **AMUD-Dashboard** and **AMUD-Agent** in Docker with **bridge** network (default), agent with `/var/run/docker.sock` mounted.
2. Label a test container with `amud.enable=true` and `amud.url`.
3. Add the app in AMUD with **Show container metrics** on.
4. Expect at least `—` CPU/RAM in the card body (static HTML). Live values need the agent to match the container name.
5. Switch agent to `network_mode: host` and compare — mirrors the Unraid fix from v1.5.5.7+.

---

## PWA / Browser Cache Issues

After upgrading AMUD, stale service worker cache can make the UI look old or keep old JavaScript loaded.

Try in this order:

```text
1. Hard refresh: Ctrl+Shift+R
2. Open DevTools → Application → Service Workers → Unregister
3. DevTools → Application → Storage → Clear site data
4. Reload the dashboard
```

If using a reverse proxy, also verify `/static/sw.js`, `/static/manifest.json`, and `/ws` are not blocked or rewritten.

---

## Media Integrations Not Showing Streams

Real Jellyfin and Plex playback requires credentials on the **app card** (Add App / Edit App → Integration), not in global settings. Credentials saved in the old **Settings → Integrations** fields are migrated to the app card automatically on upgrade.

### Jellyfin

On your Jellyfin app card, set:

- **URL**, for example `http://jellyfin.local:8096`
- **Integration**: Jellyfin
- **API key / token**: a Jellyfin API key

AMUD polls:

```text
GET /Sessions
X-Emby-Token: <api-key>
```

### Plex

On your Plex app card, set:

- **URL**, for example `http://plex.local:32400`
- **Integration**: Plex
- **API key / token**: your Plex token

AMUD polls:

```text
GET /status/sessions
Header: X-Plex-Token: <token>
```

If no credentials are configured, the stream cards show **NOT CONFIGURED** instead of fake playback.

---

## Useful Service Commands

Run from the Proxmox host. Replace `<CT_ID>` with your `amud-dashboard` container ID (see [Find Your AMUD Container ID](#find-your-amud-container-id)):

```bash
# Dashboard service inside LXC
pct exec <CT_ID> -- systemctl status amud
pct exec <CT_ID> -- systemctl restart amud
pct exec <CT_ID> -- journalctl -u amud --no-pager -n 50

# Host telemetry agent
systemctl status amud-agent
systemctl restart amud-agent
journalctl -u amud-agent --no-pager -n 50

# Confirm socket mount
grep mp0 /etc/pve/lxc/<CT_ID>.conf
ls -la /opt/amud/run
```

---

## Host telemetry mapping

**Symptom:** Disk shows **0.0 GB**, bandwidth shows **—**, numbers do not match the host, or you need help filling **Settings → Privacy & Access → Host telemetry mapping**.

**Full guide:** [Configuration — Host telemetry mapping](./configuration.md#host-telemetry-mapping) (how to find interface names and mount paths on Proxmox, Unraid, and Docker).

### Decision tree

```text
Disk 0 GB or Bandwidth — ?
├─ Admin sees (container scope) on dashboard
│  └─ Agent runs in Docker bridge mode without host mounts
│     → Unraid: Force Update agent, set Network=host, bind /mnt/user and /mnt/cache
│     → Docker Compose: network_mode: host + volume bind-mounts (see docker-compose.yml comments)
├─ Admin sees (auto-detect)
│  └─ Configured iface/mount names not visible inside agent
│     → Settings → Test host visibility; fix names or host network/mounts
├─ Mapping blank but still wrong
│  └─ Agent not connected — check agent logs / container status
└─ Proxmox: discovery must run on hypervisor host (agent runs on host, not dashboard LXC)
```

### Platform quick fixes

| Platform | Network fix | Disk fix |
|----------|-------------|----------|
| **Unraid CA** | AMUD Agent template: **Network = host** (v1.5.5.7+) | Bind `/mnt/user` and `/mnt/cache` read-only in agent template |
| **Docker Compose** | `network_mode: host` on `agent` service | `- /mnt/user:/mnt/user:ro` (adjust paths) |
| **Proxmox** | Run `cat /proc/net/dev` on **host**; map `eno1`/`bond0` external, `vmbr0` internal | `df -h` on host; e.g. `/,/var/lib/vz` |
| **Bare metal** | Auto usually works; override only if needed | Auto or explicit `df -h` paths |

**Unraid worked example (after v1.5.5.7 host network + mounts):**

```
External: eth0
Internal: br0,br0.40
Disk:     /mnt/cache,/mnt/user
```

Expect two disk tiles (**cache** and **user**) and non-zero bandwidth under load.

**Quick checks:**

1. Leave all three fields **blank** first — auto mode works for most installs.
2. Use **Settings → Test host visibility** — confirm `Scope: host` and listed ifaces/mounts match your mapping.
3. Discovery commands run on the **host where amud-agent runs** (not the dashboard container unless agent is there too):

```bash
cat /proc/net/dev    # interface names (first column)
df -h                # mount paths (Mounted on column)
```

4. Inside the **agent container** (Unraid: Docker → AMUD-Agent → Console):

```bash
cat /proc/net/dev
df -h
```

Compare to Settings. If `br0` / `/mnt/user` are missing, fix network mode and bind-mounts.

5. After saving settings, confirm values persisted (dashboard container or host with DB access):

```bash
sqlite3 /opt/amud/data/amud.db "SELECT key, value FROM settings WHERE key LIKE 'telemetry_%';"
```

6. Agent must be connected — check **AMUD-Agent** logs on Unraid or `systemctl status amud-agent` on bare metal.

If you override network interfaces, list **both** external and internal names; unlisted interfaces are not counted. If WAN uses **bond0** but you configured **eth0**, the agent may fall back to auto-detect after a few samples — try `bond0` as external instead.

---

## Audit Log Empty

**Symptom:** Settings → **Audit** shows no entries even after you sign out, sign back in, or save settings.

**Checks:**

```bash
# Row count in the database (replace <CT_ID> with your container ID)
pct exec <CT_ID> -- sqlite3 /opt/amud/data/amud.db "SELECT COUNT(*) FROM audit_log;"

# Server-side audit errors
pct exec <CT_ID> -- journalctl -u amud --no-pager -n 100 | grep -i AUDIT
```

**Common fixes:**

- **Missing `username` column** — server logs show `no column named username`. Fixed in **v1.5.4.1+** (auto-migration on start). Upgrade with `update-amud.sh` and `systemctl restart amud`, or manually: `ALTER TABLE audit_log ADD COLUMN username TEXT NOT NULL DEFAULT '';`
- **Sign out and sign back in**, then change a setting and click **Save** — only actions after v1.4+ are recorded; older history is not backfilled.
- **Database permissions** — if you restored or copied `amud.db` manually, ensure the service user can write:

```bash
pct exec <CT_ID> -- chown amud:amud /opt/amud/data/amud.db
pct exec <CT_ID> -- systemctl restart amud
```

- **Binary/UI mismatch** — run `update-amud.sh` on the Proxmox host so both `amud-server` and UI templates update together, then restart the service.

---

## Recovering from Broken Custom CSS

**Symptom:** You entered custom CSS into the Settings menu that caused the dashboard to become entirely unusable (e.g., hiding the body, breaking the grid, making buttons unclickable), and now you cannot reach the Settings menu to fix it.

**Fix:** You can reset the custom CSS directly from the SQLite database via the CLI:

```bash
pct exec <CT_ID> -- sqlite3 /opt/amud/data/amud.db "UPDATE settings SET value='' WHERE key='custom_css';"
pct exec <CT_ID> -- systemctl restart amud
```

---

## Database Restore Failure

**Symptom:** Restore shows "Import failed" even though the server restarted, or the dashboard won't come back after upload.

**Fix (v1.5.6.1+):** Restore now checkpoints WAL, removes stale `-wal`/`-shm` files, and returns success before restart. If the page says failed but AMUD restarted, hard-refresh — the restore likely worked.

**Revert a bad restore:** AMUD saves `amud.db.bak` before overwrite. Also back up `.amud-secrets-key` with your database — encrypted integration tokens need the matching key.

```bash
pct exec <CT_ID> -- mv /opt/amud/data/amud.db.bak /opt/amud/data/amud.db
pct exec <CT_ID> -- rm -f /opt/amud/data/amud.db-wal /opt/amud/data/amud.db-shm
pct exec <CT_ID> -- systemctl restart amud
```

---

## Getting Help

If your issue isn't covered here:

1. Run the diagnostic command and save the output:

```bash
journalctl -u amud-agent --no-pager -n 50 > /tmp/amud-debug.log
pct exec <CT_ID> -- journalctl -u amud --no-pager -n 50 >> /tmp/amud-debug.log
cat /tmp/amud-debug.log
```

2. Open an issue on [GitHub](https://github.com/boubli/AMUD-Dashboard/issues) with the log output and your Proxmox version.

