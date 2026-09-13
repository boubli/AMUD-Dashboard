---
sidebar_position: 2
title: Features
---

# Features

Complete inventory of what AMUD Dashboard ships today (1.9.3). Everything below is implemented in the compiled Rust binaries — no YAML files, no Node.js runtime.

![AMUD Dashboard — default theme](/img/AMUD-Dashboard.png)

### Quick demos

**Update from Settings → System** (native / Proxmox):

![Update AMUD](/img/amud-update.gif)

---

## Core dashboard

| Feature | Details |
|---------|---------|
| **Bento app grid** | Drag-and-drop reorder (admin), card spans (`1x1`, `2x1`, `1x2`), category filter tabs |
| **Dashboard widgets** | Custom note, link list, or HTML blocks above the app grid — see [Dashboard Widgets](./dashboard-widgets) |
| **~2000 bundled logos** | Offline SVG library plus custom URL icons |
| **Light & dark mode** | System-wide theme toggle with **41 bundled CSS themes** (including WebGL Taghawsa) and visual Theme Gallery |
| **Video wallpapers** | `.mp4`, `.webm`, `.ogg` background support |
| **Weather widget** | Open-Meteo via latitude/longitude in Settings |
| **Clock timezone & format** | IANA timezone + 12h/24h for the dashboard clock (Settings → Appearance) |
| **Web search + custom engines** | Built-in engines plus user-defined HTTPS `{query}` / `%s` templates; default engine selectable |
| **Live settings preview** | Accent, glass blur/opacity, wallpaper overlay strength, grid columns, wallpaper |
| **Dedicated `/feeds` page** | RSS-only view for guest-friendly news cards |
| **Featured hero headline** | Top story banner on `/feeds` |
| **Feed categories** | World News, Tech, etc. with icons and tab colors |
| **RSS reorder** | Drag feeds in Settings; order drives `/feeds` layout |
| **In-app update banner** | Native/Proxmox installs check GitHub Releases automatically |
| **PWA service worker** | Basic static asset caching (offline shell polish is on the roadmap) |

---

## Telemetry & agent (`amud-agent`)

| Feature | Details |
|---------|---------|
| **Host metrics** | CPU (model, cores, %, temperature), RAM, disk, GPU via `nvidia-smi` |
| **Network bandwidth** | Internal and external interface throughput (Mbit/s) |
| **Proxmox LXC** | Native HTTPS REST API — no `pvesh` subprocesses |
| **Docker monitoring** | Container list + CPU/RAM (`AMUD_DOCKER=1`, Unix only) |
| **LXC power controls** | Start, stop, reboot, shutdown from dashboard cards |
| **Docker power controls** | Start, stop, restart from dashboard cards |
| **Live WebSocket stream** | `/ws` pushes telemetry; role-filtered payloads for Admin vs Guest |
| **App health badges** | ONLINE / OFFLINE / BLOCKED with latency on URL checks |
| **IPC authentication** | Challenge-response with shared `AMUD_AGENT_SECRET` |

---

## App card integrations

Configure per app under **Add/Edit App → Integration**. Data loads when the card renders (on-demand fetch, not background polling).

| Integration | Data shown | Admin actions | Guest visible |
|-------------|-----------|---------------|---------------|
| **Pi-hole** | Ads blocked today, status | Disable 5 min | No |
| **AdGuard Home** | Blocked today, protection state | Disable 5 min | No |
| **Radarr** | Queue, missing, movies, disk, version, health | — | No |
| **Sonarr** | Queue, missing, series, episodes, disk, version | — | No |
| **Lidarr** | Queue, missing, artists, albums, disk, version, health | — | No |
| **Readarr** | Queue, missing, books, authors, disk, version, health | — | No |
| **Whisparr** | Queue, missing, series, episodes, disk, version, health | — | No |
| **Overseerr** | Pending media requests | — | No |
| **Jellyseerr** | Pending media requests | — | No |
| **Prowlarr** | Enabled/total indexers, queue size | — | No |
| **qBittorrent** | Download speed, active downloads, seeding | — | No |
| **SABnzbd** | Queue, download speed, free disk, version | — | No |
| **NZBGet** | Queue, download speed, free disk, version | — | No |
| **Transmission** | Download/upload speed, active torrents, free disk | — | No |
| **Jackett** | Indexers enabled/total, failed count | — | No |
| **Tautulli** | Active streams, bandwidth, libraries | — | No |
| **Audiobookshelf** | Library and item counts | — | No |
| **Immich** | Photos, videos, assets, storage | — | No |
| **Tdarr** | Staged queue, workers | — | No |
| **Maintainerr** | Issues, rules | — | No |
| **Watchtower** | Monitored containers | — | No |
| **Ombi** | Pending media requests | — | No |
| **FileBrowser** | Health status | — | No |
| **Frigate** | Cameras up/total, detection FPS | — | No |
| **Bazarr** | Missing subtitles (episodes/movies) | — | No |
| **Uptime Kuma** | Monitors up/down | — | No |
| **Cloudflare Tunnel** | Tunnel status, connections | — | No |
| **Peanut (UPS)** | Battery %, UPS status | — | No |
| **FRITZ!Box** | WAN status, speeds, external IP, uptime, devices, model, link rates | — | No |
| **Portainer** | Containers running/stopped, stacks, endpoints, version | — | No |
| **OPNsense** | CPU, memory, states, gateways, uptime, version | — | No |
| **pfSense** | CPU, memory, states, uptime, version | — | No |
| **TrueNAS** | Pool health, storage used/free, version | — | No |
| **UniFi Network** | WAN status, clients, APs online, devices, latency | — | No |
| **Grafana** | Dashboards, datasources, org, version | — | No |
| **Netdata** | Host CPU, alarms, charts, version | — | No |
| **Glances** | Host CPU, RAM, load | — | No |
| **Beszel** | Systems up/down, average CPU | — | No |
| **Paperless-ngx** | Documents, inbox, correspondents, tags, storage | — | No |
| **Mealie** | Recipes, users, version | — | No |
| **Nextcloud** | Active users, free space, version | — | No |
| **Vaultwarden** | Users, organizations, version | — | No |
| **Deluge** | Downloading, seeding, torrents, free disk | — | No |
| **Navidrome** | Artists, version | — | No |
| **Komga** | Series, books, libraries | — | No |
| **PhotoPrism** | Photos, videos, albums, index status | — | No |
| **Proxmox VE** | Nodes, VMs, LXCs, cluster CPU/RAM, version | — | No |
| **Tailscale** | Devices online, exit nodes | — | No |
| **Netbird** | Peers, connected, setup keys | — | No |
| **Ollama** | Models installed, running models | — | No |
| **Open WebUI** | Models (with API key), health | — | Optional |
| **Synology DSM** | Version, model, volumes, uptime | — | No |
| **Unraid** | Array state, parity, used slots | — | No |
| **Dockge** | Compose stacks | — | No |
| **Nginx Proxy Manager** | Proxy hosts, certificates | — | No |
| **Traefik** | Routers, services, middlewares | — | No |
| **Authentik** | Users, flows | — | No |
| **Authelia** | Health, version | — | No |
| **CrowdSec** | Alerts, decisions | — | No |
| **Node-RED** | Flows, version | — | No |
| **Zigbee2MQTT** | Devices | — | No |
| **Home Assistant** | Entities, lights on, version | — | No |
| **Emby** | Sessions, version, server name | — | No |
| **Scrypted** | Plugins | — | No |
| **Mylar / Kapowarr / Huntarr** | Comics *arr stats | — | No |
| **Proxmox Backup Server** | Datastores | — | No |
| **Technitium DNS** | Zones | — | No |
| **Blocky DNS** | Blocking status | — | No |
| **OpenWrt** | LuCI reachability (limited) | — | No |
| **RSS / Atom** | Top 3 headlines | — | **Yes** |
| **Plex / Jellyfin** | Per-app streams, status | — | No |
| **Custom API** | Up to 6 mapped JSON fields | — | No |
| **Autobrr / Gotify / Prometheus / OMV / FreshRSS / ntfy / Coolify / Aria2 / Kubernetes** | Standard metric cards | — | No |

### Migration & parity

- **Homepage YAML import** — `services.yaml` → SQLite apps ([migration guide](./migration/homepage.md))
- **Docker Homepage labels** — discovered via `amud-agent`
- **Comparison matrix** — [AMUD vs Homepage vs Homarr](./comparison.md)

### Health-check integrations (1×1 link card)

Status and version/latency only — use integration type with API token or `none` for URL health probe:

Gitea, Forgejo, GitLab, Jenkins, Drone, MinIO, Garage, SeaweedFS, Kopia, Restic, Duplicati, UrBackup, Kodi, Stash, Channels DVR, Calibre-web, Headscale, WireGuard UI, OpenVPN, Hubitat, SmartThings, ioBroker, Blue Iris, Shinobi, Agent DVR.

RSS feeds are managed under **Settings → RSS Feeds** (not the Add App modal).

---

## Media stream cards

Configured per app (**Add App / Edit App → Integration** with an API key or token). Polls every few seconds when a matching app card exists.

| Service | Stream card |
|---------|-------|
| **Jellyfin** (incl. Emby) | Now playing title, **movie poster**, progress bar, multi-stream summary, admin **pause / resume / stop** controls (shown only while streaming) |
| **Plex** | Now playing title, progress bar, multi-stream summary |

Posters are proxied through the AMUD server so the media server URL and API key never reach the browser.

---

## Smart home

| Feature | Details |
|---------|---------|
| **Home Assistant** | Lights on, switches on, average temperature on a card named exactly `Home Assistant` |
| **Template API** | Lightweight `POST /api/template` with `/api/states` fallback |

---

## Auth & users

| Feature | Details |
|---------|---------|
| **Roles** | Admin (full control) and Guest (read-only dashboard) |
| **Argon2id passwords** | Legacy SHA-256 hashes rehashed on login |
| **Bootstrap admin** | Random one-time password printed on first boot |
| **Sessions** | 24h HttpOnly cookies, CSRF tokens, login rate limiting |
| **OIDC SSO** | OpenID Connect login with configurable issuer, client, redirect |
| **LDAP** | Optional bind authentication for homelab directories |
| **API tokens** | Scoped Bearer tokens (`read:apps`, `read:status`, `read:telemetry`, `read:integrations`, `write:webhooks`, …) for automation |
| **Encrypted secrets** | Integration API keys stored with ChaCha20-Poly1305 at rest |
| **Security headers** | CSP nonce, X-Frame-Options, HSTS-ready via reverse proxy |

---

## Webhooks & notifications

| Event | Payload formats |
|-------|-----------------|
| `container_started` / `container_stopped` | Discord embed, Telegram HTML, generic JSON |
| `agent_connected` / `agent_disconnected` | Same |

Manage under **Settings → Webhooks**. Test button included. SSRF protection blocks loopback and private IPs (homelab LAN targets may need a public relay).

---

## Wake-on-LAN

Dedicated **Settings → Wake-on-LAN** device list (decoupled from app cards). Send UDP magic packets from dashboard cards or the WOL manager.

---

## Backup & restore

| Action | Endpoint |
|--------|----------|
| Export `amud.db` | Settings → Backup (streaming download for large databases) |
| Import `amud.db` | Validates SQLite header, creates `.bak`, restarts process |
| Backup reminder | Optional banner when last export is older than configured days |

Back up `.amud-secrets-key` alongside the database when using encrypted integration tokens.

---

## Audit log

24 action types tracked (login, app CRUD, webhooks, backup, system updates, container actions, etc.). View and filter under **Settings → Audit**.

Not yet audited: card reorder, category CRUD, wake packets.

---

## System & deployment

| Install path | Docs |
|--------------|------|
| Proxmox LXC one-liner | [Proxmox](./installation/proxmox.md) |
| Docker Compose (server + agent) | [Docker](./installation/docker.md) |
| Bare-metal Linux | [Linux](./installation/linux.md) |
| Unraid Community Apps | [Unraid](./installation/unraid.md) |
| Hydrivax | [Hydrivax](./installation/hydrivax.md) |

In-app updater works on native/Proxmox installs only (Docker uses image pulls).

---

## See also

- [Configuration](./configuration.md) — appearance, media, Home Assistant, Proxmox
- [Roadmap](./roadmap.md) — what is next
- [Changelog](./changelog.md) — release history
