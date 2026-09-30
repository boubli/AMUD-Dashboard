---
sidebar_position: 6
title: Unraid
description: Install AMUD Dashboard on Unraid via Community Applications — dashboard + agent templates.
---

# Unraid Installation

AMUD ships as **two** Community Applications templates (dashboard + telemetry agent). This matches the Docker Compose architecture and keeps the agent isolated with read-only Docker socket access.

---

## Prerequisites

- Unraid 6.9+ with the **Community Applications** plugin installed
- A free TCP port (default **8000**) for the web UI
- ~100MB free RAM for both containers combined

---

## Step 1 — Install from Community Applications

After the templates are published on CA (or while testing from the maintainer repository):

1. Open the Unraid **Apps** tab.
2. Search for **AMUD Dashboard** and click **Install**.
3. Set a strong **Agent Secret** (long random string). **Copy it** — you need the same value on the agent.
4. Leave default paths unless you use a custom appdata layout:
   - App Data: `/mnt/user/appdata/amud-dashboard/data`
   - Agent Socket Dir: `/mnt/user/appdata/amud-dashboard/run`
5. Click **Apply** and wait for the container to start.

6. Search for **AMUD Agent** and click **Install**.
7. Paste the **same Agent Secret** as step 3.
8. Confirm **Agent Socket Dir** matches the dashboard (`/mnt/user/appdata/amud-dashboard/run`).
9. Set **Docker Monitoring** to `1` (recommended on Unraid).
10. Click **Apply**.

---

## Step 2 — First login

1. Open `http://YOUR_UNRAID_IP:8000` (or your reverse-proxy URL).
2. On first boot, the server prints a one-time **admin password** in the container log:
   - Unraid → **Docker** → **AMUD-Dashboard** → **Log**
3. Log in as `admin` with that password and change it under **Settings**.

---

## Step 3 — Verify telemetry

Within a few seconds of both containers running:

- Host CPU/RAM/disk widgets should populate.
- With `AMUD_DOCKER=1` on the agent, Docker container cards can show live status.

If telemetry is missing:

- Confirm both containers are **running**.
- Confirm **Agent Secret** matches on both (re-create if unsure).
- Confirm **Agent Socket Dir** is identical on both.
- Check **AMUD-Agent** logs for connection errors.

See [Permission errors on appdata](#permission-errors-on-appdata) below if the dashboard container exits or the database cannot be created.

To customize which network interfaces or disk mounts appear on the dashboard, see [Host telemetry mapping](../configuration.md#host-telemetry-mapping) (Unraid example: `/mnt/user` for array-only storage).

---

## Step 3b — Host network and disk bind-mounts (v1.5.5.7+)

**Why:** On Unraid, the AMUD Agent container used **bridge** networking by default. Bridge mode hides host interfaces (`br0`, `bond0`) and host disk paths (`/mnt/user`, `/mnt/cache`) from the agent. If you set host interface names in Settings but see **Bandwidth —** or **Disk 0.0 GB**, this is the usual cause.

The **AMUD Agent** CA template (v1.5.5.7+) uses **host** network and optional read-only bind-mounts:

| Template field | Default | Purpose |
|----------------|---------|---------|
| **Network** | `host` | Agent reads host `/proc/net/dev` (sees `br0`, `bond0`, etc.) |
| **Array mount** | `/mnt/user:/mnt/user:ro` | Array storage for disk telemetry |
| **Cache mount** | `/mnt/cache:/mnt/cache:ro` | Cache pool for disk telemetry |

**Upgrade steps:**

1. **Force Update** both **AMUD-Dashboard** and **AMUD-Agent** to `v1.5.5.7` (or `latest`).
2. Re-apply the agent template so **Network** is **host** and array/cache paths are mapped (or add them manually under **Extra Parameters** / path mappings).
3. Hard-refresh the dashboard (`Ctrl+Shift+R`).
4. Open **Settings → Privacy & Access → Host telemetry mapping** and click **Test host visibility**. You should see `Scope: host` and your Unraid interfaces/mounts listed.

**Example mapping (Inch-high / typical Unraid):**

```
External network interfaces:  eth0
Internal network interfaces:  br0,br0.40
Disk mount points:            /mnt/cache,/mnt/user
```

With both array and cache configured, the dashboard shows **separate disk tiles** for each mount (v1.5.5.7+).

**Still on bridge network?** The agent reports `scope: container` and admins see **(container scope)** on Disk/Bandwidth cards. Switch the agent to host network or leave mapping blank for auto-detect inside the container.

---

## Permission errors on appdata

**Symptom:** The **AMUD-Dashboard** container fails to start, restarts in a loop, or logs show **permission denied** when writing to `/app/data`, creating `.amud-secrets-key`, or when the agent cannot use the shared socket under `run/`.

See also: [Unraid: `.amud-secrets-key` permission denied](../troubleshooting.md#unraid-secrets-key-permission-denied) for the exact first-boot error from Community Applications.

**Cause:** Unraid maps host paths into the container. If `data/` or `run/` on the host are not writable by the user the dashboard process runs as, SQLite and the agent Unix socket will fail.

Default CA paths:

| Path | Purpose |
|------|---------|
| `/mnt/user/appdata/amud-dashboard/data` | SQLite database, `.amud-secrets-key`, settings |
| `/mnt/user/appdata/amud-dashboard/run` | Shared socket between dashboard and agent |

**v1.7.2+ (recommended):** The dashboard image runs as **PUID 99 / PGID 100** inside the container (matches Unraid `nobody:users` appdata). After updating to the latest image, **recreate** the dashboard container — no SSH `chown` required on a fresh install.

**Fix 1 — Manual ownership (older images or custom paths)**

If you cannot update yet, or use a custom appdata layout, align host ownership with the container user:

```bash
# v1.7.2+ dashboard (PUID 99 — Unraid default)
chown -R 99:100 /mnt/user/appdata/amud-dashboard/data
chown -R 99:100 /mnt/user/appdata/amud-dashboard/run
chmod -R 755 /mnt/user/appdata/amud-dashboard/data
chmod -R 770 /mnt/user/appdata/amud-dashboard/run
```

```bash
# Legacy images (root in container)
chown -R 0:0 /mnt/user/appdata/amud-dashboard/data
chown -R 0:0 /mnt/user/appdata/amud-dashboard/run
chmod -R 755 /mnt/user/appdata/amud-dashboard/data
chmod -R 770 /mnt/user/appdata/amud-dashboard/run
```

Then restart **AMUD-Dashboard**, then **AMUD-Agent**.

---

## Reset admin password (Docker)

If you cannot log in, reset the password from the Unraid terminal (legacy SHA-256 hash; AMUD upgrades it to Argon2id on next login):

```bash
# 1. Generate SHA256 hash of your new password
echo -n 'YOUR_NEW_PASSWORD' | sha256sum | awk '{print $1}'

# 2. Update the database (replace HASH with output above)
docker run --rm -v /mnt/user/appdata/amud-dashboard/data:/data alpine sh -c \
  "apk add --no-cache sqlite >/dev/null 2>&1 && sqlite3 /data/amud.db \"UPDATE users SET password_hash='HASH' WHERE username='admin';\""
```

Restart the **AMUD-Dashboard** container, sign in, then change the password under **Settings → Security** so a fresh Argon2id hash is stored.

See also [Troubleshooting — Reset admin password](../troubleshooting.md#reset-or-change-the-admin-password-from-cli).

---

**Fix 2 — Custom PUID/PGID**

The image defaults to UID **99** / GID **100**. If you use different values in the template, you must also set matching **Extra Parameters** on the dashboard container:

```
--user YOUR_PUID:YOUR_PGID
```

Host appdata ownership must match:

```bash
chown -R YOUR_PUID:YOUR_PGID /mnt/user/appdata/amud-dashboard/data
chown -R YOUR_PUID:YOUR_PGID /mnt/user/appdata/amud-dashboard/run
```

(Replace `YOUR_PUID:YOUR_PGID` with your chosen IDs, e.g. `99:100`.)

**Verify**

```bash
ls -la /mnt/user/appdata/amud-dashboard/
```

Both `data` and `run` should be owned by the same UID/GID the container uses. After fixing, open the UI — host telemetry should populate within a few seconds.

---

## Reverse proxy

AMUD uses WebSockets for live telemetry. If you put it behind Nginx Proxy Manager or Traefik, enable WebSocket upgrade on the backend. See [Reverse Proxy](./reverse-proxy.md).

---

## Updating

1. Stop **AMUD-Agent**, then **AMUD-Dashboard** (optional but avoids a brief socket race).
2. In **Apps**, click the container icon → **Force Update** (or set tag to `latest` and recreate).
3. Update **both** containers to the same image tag.
4. Start dashboard, then agent.

Your config lives in `/mnt/user/appdata/amud-dashboard/data` — back up that folder before major upgrades.

---

## Support

Please use the right channel so reports are tracked and fixed in releases:

| Channel | Use for |
|---------|---------|
| [**GitHub Issues**](https://github.com/boubli/AMUD-Dashboard/issues) | **Bugs**, install failures, feature requests — include Unraid version, container logs, and steps to reproduce |
| [GitHub Discussions](https://github.com/boubli/AMUD-Dashboard/discussions) | Questions, screenshots, general homelab chat |
| [Documentation](https://boubli.github.io/AMUD-Dashboard/docs/troubleshooting) | Self-service fixes (permissions, WebSockets, agent socket) |

Unraid forum threads are welcome for visibility, but **open a GitHub Issue** for anything that needs a code or docs fix so it is not lost in the thread.

---

## Additional Unraid / remote hosts

Install only the **AMUD Agent** on other machines. Point `AMUD_SERVER_ADDR` at your AMUD dashboard host, set a unique `AMUD_NODE_TAG`, and reuse the agent secret. On the dashboard set `AMUD_AGENT_TCP_LISTEN=0.0.0.0:8050`. Prefer VPN or TLS — see [Remote agents / multi-node](./remote-agents).
