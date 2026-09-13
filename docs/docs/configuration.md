---
sidebar_position: 2
---

# Dashboard Configuration

Most AMUD settings are managed in the web UI under **Settings** (admin login required). This page covers appearance layout and media integrations.

---

## Appearance

Open **Settings → Appearance** to customize the dashboard look and layout.

### Grid columns

The **Grid columns** dropdown controls how many app cards appear per row on desktop screens:

| Value | Use case |
|-------|----------|
| 2 | Large cards, fewer columns |
| 3 | Default balanced layout |
| 4 | Dense homelab with many services |
| 5 | Maximum density on wide monitors |

The setting is stored as `grid_columns` in the SQLite settings table (default: `3`). On smaller viewports, the layout automatically reflows to fewer columns for mobile and tablet screens.

Other appearance options on the same tab include accent color, glass blur/opacity, background image, logo, and bento card radius.

### Clock

**Timezone** (`clock_timezone`) — Browser local (default) or an IANA zone such as `Europe/Paris`. Drives the dashboard clock, date line, and greeting.

**Time format** (`clock_time_format`) — `12h` (AM/PM) or `24h`.

### Web search

**Default search engine** (`default_search_engine`) — built-in (Google, Bing, DuckDuckGo, YouTube, GitHub) or a custom id.

**Custom engines** (`custom_search_engines`) — JSON list of `{ "id", "name", "url" }` where `url` is `https://…` and includes `{query}` or `%s`. Managed from Settings → Appearance → Web search.

---

## Dashboard widgets

Custom **note**, **link list**, and **HTML** blocks can be added above the main app grid under **Settings → Account → Widgets**.

Each widget supports bento spans (`1×1`, `2×1`, `1×2`) and an optional **guest visible** toggle. Copy-paste examples for Note, Links, and HTML are in the dedicated guide:

**[Dashboard Widgets](./dashboard-widgets)**

---

## Media integrations (Jellyfin & Plex)

Media servers are configured **per app** — the same place as every other integration. Add (or edit) your Jellyfin, Emby, or Plex app card and pick the matching **Integration** in the dropdown, then paste the API key or token. The "Now Playing" stream card, movie posters, and playback controls follow that app automatically.

:::info Migrating from older versions
Before v1.8.1, Jellyfin and Plex credentials lived in **Settings → Integrations**. On first start, AMUD automatically moves those credentials into the matching app card, so no manual action is needed.
:::

When credentials are missing, stream badges show **NOT CONFIGURED**. When configured, badges start as **CHECKING...** and update from live session polling plus LXC/Docker telemetry.

### Jellyfin / Emby

| Field (in Add App / Edit App) | Description |
|-------|-------------|
| **URL** | Base URL of your server, e.g. `http://jellyfin.local:8096` |
| **Integration** | Select **Jellyfin** |
| **API key / token** | API key from Jellyfin admin |

**Create an API key in Jellyfin:**

1. Open Jellyfin as an administrator.
2. Go to **Dashboard → Advanced → API Keys**.
3. Click **+** to create a key (e.g. name it `AMUD`).
4. Paste the key into the app's **API key / token** field.

AMUD polls:

```http
GET /Sessions
X-Emby-Token: <your-api-key>
```

The API key is sent in the `X-Emby-Token` header (Emby/Jellyfin convention), not as a query parameter.

While someone is streaming, the Jellyfin card shows the **movie/episode poster** (proxied through the AMUD server — the API key never reaches the browser) plus **pause / resume / stop** controls for admins. When nothing is playing, the poster and controls are hidden automatically.

### Plex

| Field (in Add App / Edit App) | Description |
|-------|-------------|
| **URL** | Base URL of your Plex server, e.g. `http://plex.local:32400` |
| **Integration** | Select **Plex** |
| **API key / token** | `X-Plex-Token` for your account |

**Find your Plex token:**

1. Sign in to [plex.tv](https://app.plex.tv) and open your server, **or**
2. Use Plex's documented token claim flow for your account, **or**
3. Inspect an authenticated Plex web request in browser DevTools for the `X-Plex-Token` header.

AMUD polls:

```http
GET /status/sessions
X-Plex-Token: <your-token>
```

When multiple clients are streaming, the badge may show the primary title plus `(+N more)`.

---

## App card integrations

Per-app integrations are set when you **Add** or **Edit** an app (Integration dropdown). AMUD fetches data when the card loads.

| Integration | App URL field | Credential field | Notes |
|-------------|---------------|------------------|-------|
| **Pi-hole** | Pi-hole web UI base URL | Web password / API token | Shows ads blocked today; admin can disable 5 min |
| **AdGuard Home** | AdGuard UI base URL | Basic auth credential | **Not an API key.** Base64-encoded `username:password` for the AdGuard UI login (see below) |
| **Radarr** | Radarr base URL | API key (`X-Api-Key`) | Queue, missing, movies, disk, version, health |
| **Sonarr** | Sonarr base URL | API key | Queue, missing, series, episodes, disk, version |
| **Lidarr** | Lidarr base URL | API key | Queue, missing, artists, albums, disk, version, health |
| **Readarr** | Readarr base URL | API key | Queue, missing, books, authors, disk, version, health |
| **Whisparr** | Whisparr base URL | API key | Queue, missing, series, episodes, disk, version, health |
| **Overseerr** | Overseerr base URL | API key | Pending media requests |
| **Jellyseerr** | Jellyseerr base URL | API key | Pending media requests |
| **Prowlarr** | Prowlarr base URL | API key (`X-Api-Key`) | Enabled/total indexers + queue size |
| **qBittorrent** | Web UI base URL | `username\|password` | Download speed, active downloads, seeding (hover card) |
| **SABnzbd** | SABnzbd base URL | API key | Queue, download speed, free disk, paused, version |
| **NZBGet** | NZBGet base URL | `username\|password` | Queue, download speed, free disk, paused, version |
| **Transmission** | RPC/Web base URL | `username\|password` (optional) | Download/upload speed, active torrents, free disk |
| **Jackett** | Jackett base URL | API key (`X-Api-Key`) | Indexers enabled/total, failed count, version |
| **Tautulli** | Tautulli base URL | API key | Active streams, bandwidth, libraries (hover card) |
| **Audiobookshelf** | Audiobookshelf base URL | Bearer API token | Library and item counts |
| **Immich** | Immich base URL | API key (`x-api-key`) | Photos, videos, assets, storage used |
| **Tdarr** | Tdarr server URL | — (optional) | Staged queue, workers, health |
| **Maintainerr** | Maintainerr base URL | API key | Issue and rule counts |
| **Frigate** | Frigate base URL | API key (optional) | Cameras up/total, detection FPS |
| **Bazarr** | Bazarr base URL | API key | Missing subtitle counts for episodes and movies (hover card) |
| **Uptime Kuma** | Uptime Kuma base URL | Status page slug **or** API key | Monitors up/down (status page JSON or `/api/monitors`) |
| **Cloudflare Tunnel** | Tunnel hostname or dashboard URL | `account_id\|tunnel_id\|api_token` | Tunnel status + active connections |
| **Peanut (UPS)** | Peanut/NUT base URL | API token (optional) | Battery % and UPS status |
| **RSS / Atom** | — | — | Manage under **Settings → RSS Feeds**; top 3 headlines on `/feeds`; **visible to guests** |

RSS feeds are not added via the dashboard **Add App** modal — use **Settings → RSS Feeds** (stored as `integration_type=rss` apps).

### AdGuard Home credential (not an API key)

AdGuard Home uses **HTTP Basic authentication**, not a Pi-hole-style API token. In the Add/Edit app form:

1. Set **Integration** to **AdGuard Home**.
2. Set **URL** to your AdGuard UI base (e.g. `http://192.168.1.10:3000`).
3. In **Basic auth credential**, paste a **Base64-encoded** `username:password` string (the same credentials you use to log into AdGuard).

Generate the value on Linux/macOS:

```bash
echo -n 'admin:your-adguard-password' | base64
```

Paste the output (e.g. `YWRtaW46eW91ci1hZGd1YXJkLXBhc3N3b3Jk`) into the credential field.

On Windows PowerShell:

```powershell
[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes('admin:your-adguard-password'))
```

The card shows blocked queries today and whether protection is enabled.

Enable **Accept invalid TLS certificates** under **Settings → Privacy & Access** if your homelab services use self-signed HTTPS (applies to Jellyfin, Plex, app integrations, and Home Assistant).

---

## Host telemetry mapping

**amud-agent** reads disk and network stats from the **host it runs on**. This works the same on **Proxmox**, **Unraid**, **Docker** hosts, bare-metal **Linux**, and most other setups — as long as the agent is installed on the machine that owns the disks and NICs you care about (not inside a random LXC/UI container unless the agent runs there too).

Under **Settings → Privacy & Access → Host telemetry mapping**, you can override what the dashboard shows:

| Setting | Example | Behavior |
|---------|---------|----------|
| **External network interfaces** | `eth0,enp3s0` | Count only these interfaces toward external bandwidth |
| **Internal network interfaces** | `vmbr0,br-0` | Count only these toward internal bandwidth |
| **Disk mount points** | `/,/mnt/user` | Sum storage usage only from these mounts |

Leave fields **blank** for automatic detection (bridges/`br*`/`docker*` → internal; physical NICs and bonds → external; all eligible disks → storage bar). If configured mounts or interfaces are **not all visible** inside the agent (common on Unraid Docker without host network or bind-mounts), the agent **falls back to auto-detect**. Admins see an **(auto-detect)** hint on the Disk and Bandwidth cards when this happens. When the agent runs in a Docker container without host network, `telemetry_scope` is **container** and admins see **(container scope)** — fix by using host network and bind-mounting disk paths (see [Unraid Step 3b](./installation/unraid.md#step-3b--host-network-and-disk-bind-mounts-v1557)).

When multiple disk mounts are configured **and** all are visible to the agent, the dashboard shows **one tile per mount** (e.g. separate **user** and **cache** cards on Unraid) plus aggregate fields for compatibility.

Names and paths are **case-insensitive**; duplicates and extra spaces are cleaned up when you save (v1.5.5.6+).

Changes push to the connected agent when you save settings.

### Supported platforms (quick reference)

| Platform | Where to run discovery | Disk mounts (examples) | Network (examples) |
|----------|------------------------|------------------------|---------------------|
| **Proxmox VE** | Proxmox **host** (not the AMUD LXC unless agent is in that LXC) | `/` OS disk, `/var/lib/vz` VM storage | `eno1` / `eth0` → external; `vmbr0` → internal |
| **Unraid** | Unraid **terminal** / host where AMUD Agent runs | `/mnt/user` full array, `/mnt/cache` cache pool only | Main NIC (`eth0`, `bond0`) → external; `br0`, `docker0`, `br-*` → internal |
| **Docker / Compose** | Machine with `docker.sock` mounted into the agent | Host root `/` or your data path | Host WAN NIC → external; `docker0`, `br-*` → internal |
| **Bare metal / VM Linux** | SSH or console on that host | `/` or e.g. `/mnt/data` | Any NIC from `/proc/net/dev` (skip `lo`) |
| **Other OS** | Same rule: run commands on the **agent host** | Any path from `df -h` | Any interface from `/proc/net/dev` |

If you are unsure, leave all three fields **empty** first. Only override when the Disk or Bandwidth widgets do not match what you expect.

### How to find your interface names and mount paths

The **amud-agent** runs on the **host** (Proxmox bare metal, Unraid, Docker host, or any Linux box). Run the discovery commands **on that host**, not inside the AMUD dashboard container/LXC unless the agent also runs there.

#### Find network interface names

```bash
cat /proc/net/dev
```

The first column is the interface name (ignore `lo`). Common patterns:

| Interface | Typical role | Auto mode |
|-----------|--------------|-----------|
| `eno1`, `enp3s0`, `eth0`, `bond0` | Physical NIC / bond (WAN/LAN) | **External** |
| `vmbr0` | Proxmox bridge | **Internal** |
| `br0`, `br0.40`, `br-*`, `docker0` | Unraid / Docker bridges | **Internal** |

**Proxmox:** run on the **hypervisor host**. Physical NIC → **External**; `vmbr0` → **Internal**.

```bash
# On the Proxmox host
cat /proc/net/dev
# Example result: eno1 (external), vmbr0 (internal)
```

**Unraid:** run in the **Unraid terminal** (or wherever **AMUD Agent** runs). Array totals → `/mnt/user`; cache-only → `/mnt/cache`. Main NIC toward your router → **External**; Docker `br-*` / `docker0` → **Internal** if you want container traffic split out.

```bash
# On Unraid (host terminal)
cat /proc/net/dev
df -h | grep -E '/mnt|Filesystem'
# Example: eth0 (external), br0 (internal), disk: /mnt/user
```

**Docker / Portainer / Compose:** the agent needs the **host** network and disk view. Interface names and mounts come from the **Docker host**, not from inside the dashboard app container. Mount `docker.sock` into the agent container and run discovery on the host (or `nsenter` / host shell).

**Other Linux / VMs:** same commands everywhere — `cat /proc/net/dev` for NICs, `df -h` for mounts. Pick the paths and interfaces that match how **you** want the dashboard summarized.

#### Find disk mount paths

```bash
df -h
```

Use the **Mounted on** column. Pick the paths you want the storage bar to represent.

| Platform | Common mounts | Example setting |
|----------|---------------|-----------------|
| **Proxmox** | OS disk + VM/CT storage | `/,/var/lib/vz` |
| **Unraid** | Array / cache | `/mnt/user` or `/mnt/cache` |
| **Generic Linux** | Root only | `/` |

AMUD skips virtual/temporary filesystems (`tmpfs`, `overlay`, etc.) automatically when summing disks.

#### Fill in Settings

1. Open **Settings → Privacy & Access → Host telemetry mapping**.
2. Paste comma-separated names/paths (no spaces required, but spaces after commas are fine).
3. Click **Save Settings** at the bottom of the page.
4. Wait ~5–10 seconds — the agent picks up the new config on the next sync.

**Example (typical Proxmox):**

```
External network interfaces:  eno1
Internal network interfaces:  vmbr0
Disk mount points:            /,/var/lib/vz
```

**Example (Unraid):**

```
External network interfaces:  eth0
Internal network interfaces:  br0
Disk mount points:            /mnt/user
```

**Example (Docker host):**

```
External network interfaces:  eth0
Internal network interfaces:  docker0,br-1234
Disk mount points:            /
```

#### Rules and tips

- **Blank = auto** — start here; only override if the dashboard disk or network numbers look wrong.
- **If you set either network list**, only interfaces you list are counted. Unlisted interfaces are ignored — fill **both** external and internal lists when overriding. If a listed interface name does not exist on the host (e.g. `eth0` vs `bond0`), the agent falls back to auto-detect.
- **Disk mounts** must all be visible inside the agent process. On Unraid Docker, use **host network** on the agent and bind-mount host paths (e.g. `- /mnt/user:/mnt/user:ro`) or leave disk mapping blank for auto-detect. See [Unraid Step 3b](./installation/unraid.md#step-3b--host-network-and-disk-bind-mounts-v1557).
- **Test host visibility** — **Settings → Privacy & Access → Host telemetry mapping → Test host visibility** asks the agent which interfaces and mounts it sees (`scope: host` vs `container`).
- **Troubleshooting inside the agent container:**

```bash
df -h
cat /proc/net/dev
```

Compare output to your Settings values. Restart the agent after changing mapping.
- **Verify saved values** (optional, admin shell on the AMUD server):

```bash
sqlite3 /opt/amud/data/amud.db "SELECT key, value FROM settings WHERE key LIKE 'telemetry_%';"
```

- **Still wrong?** See [Troubleshooting — Host telemetry mapping](./troubleshooting.md#host-telemetry-mapping).

---

## Smart Home Integration (Home Assistant)

Connect your dashboard to Home Assistant to view live sensor telemetry on the Home Assistant app card.

1. Open **Add App** (or Edit an existing app).
2. Set **Integration** to **Home Assistant**.
3. Enter your **Home Assistant URL** (e.g. `http://homeassistant.local:8123`) as the app URL.
4. Paste your **Long-Lived Access Token** in the API key field (created from your user profile in HA).
5. Save the app.

AMUD prefers credentials from the first Home Assistant–integrated app. Legacy `ha_url` / `ha_token` settings (if still present in the database) are used only as a fallback. Poll interval remains under **Settings → Performance**.

The card telemetry includes active lights, switches, and average home temperature. AMUD polls using the lightweight **Template API** (`POST /api/template`), falling back to `/api/states` when template rendering is unavailable.

---

## Custom CSS Injection

Make the dashboard truly yours by overriding the default styling.

1. Open **Settings → Appearance → Custom CSS**.
2. Open **Settings → Appearance → Theme Gallery** on your dashboard, or copy CSS from the [Theme Gallery](/themes).
3. Paste into the Custom CSS field and click **Save**. Changes apply immediately for all users.

*(Note: If invalid CSS breaks the layout, see [Troubleshooting](./troubleshooting.md) for recovery.)*

Browse **41 themes** with preview screenshots on the **[Theme Gallery](/themes)** — use the in-dashboard gallery for one-click preview, or copy CSS/wallpaper from the site. See [CSS variable reference](./themes.md).

---

## Proxmox and container control

Live **RUNNING** / **STOPPED** badges and start/stop controls require:

1. A valid **Proxmox API token** under **Settings → Proxmox VE**
2. Matching **container IDs** or Docker names on app cards
3. A working **amud-agent** on the hypervisor host

See [Proxmox VE Installation](./installation/proxmox.md#5-proxmox-api-token-configuration) for token setup.

### Per-app CPU / RAM row

Each app card can show live **CPU** and **RAM** from the host agent when the container name matches. For services that run on **another server** (or cloud), those numbers are misleading.

When adding or editing an app, use **Show CPU / RAM from host agent**:

- **On** (default) — card shows agent metrics when a container match exists
- **Off** — status badge still updates; CPU/RAM row is hidden

Guests never see per-card CPU/RAM — only **ONLINE** / **OFFLINE** availability.

---

## Environment variables

These are set on the **server** or **agent** process (Docker `environment:`, systemd unit, or shell). Most day-to-day options live in the SQLite settings table via the UI.

| Variable | Component | Default | Description |
|----------|-----------|---------|-------------|
| `PORT` | Server | `8000` | HTTP listen port |
| `BIND_ADDR` | Server | `127.0.0.1` | Bind address. Use `0.0.0.0` in Docker so the container accepts external traffic. |
| `DB_PATH` | Server | `data/amud.db` | SQLite database file path |
| `AMUD_SECRETS_KEY` | Server | auto-generated file | 32-byte key (base64url or 64-char hex) for encrypting integration tokens at rest in SQLite. If unset, AMUD writes `data/.amud-secrets-key` on first boot — back it up with `amud.db`. |
| `AMUD_AGENT_SECRET` | Both | *(required)* | Shared secret for agent ↔ server IPC authentication |
| `AMUD_SOCKET_PATH` | Both | `/var/run/amud/amud.sock` | Unix socket path for agent IPC |
| `AMUD_ENABLE_PROXMOX` | Server | `false` | Set `true` on a Proxmox LXC host to show the Proxmox settings tab |
| `AMUD_DOCKER` | Agent | auto | Enabled when `/var/run/docker.sock` is mounted. Set `0` to disable Docker monitoring and container controls. |
| `PVE_NODE` | Agent | hostname | Proxmox node name for LXC API calls when it differs from `/etc/hostname` |
| `PVE_API_TOKEN` | Agent | *(none)* | Proxmox API token; prefer setting on the agent host instead of over IPC |

### Public telemetry

In **Settings → Privacy & Access**, the **Guest system telemetry visibility** toggle stores `telemetry_public` in SQLite. When enabled, anonymous visitors and Guest-role users see host CPU model, usage, memory, GPU (when reported by the agent), and network stats on the dashboard. Container names, VMIDs, per-app metrics, streams, and admin controls stay hidden.

---

## Troubleshooting configuration issues

| Symptom | See |
|---------|-----|
| Streams show NOT CONFIGURED | [Media Integrations](./troubleshooting.md#media-integrations-not-showing-streams) |
| Apps stuck on CHECKING... | [Troubleshooting](./troubleshooting.md) — Proxmox token and agent IPC |
| Old UI after upgrade | [PWA / Browser Cache](./troubleshooting.md#pwa--browser-cache-issues) |
| Dashboard UI is broken | [Custom CSS Recovery](./troubleshooting.md#recovering-from-broken-custom-css) |
