# Changelog

All notable changes to AMUD Dashboard are documented here and on the docs site.

**Full history (readable):** https://boubli.github.io/AMUD-Dashboard/docs/changelog  
**Latest release + binaries:** https://github.com/boubli/AMUD-Dashboard/releases/latest  
**Roadmap:** https://boubli.github.io/AMUD-Dashboard/docs/roadmap

---

## [1.9.3](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.9.3) — 2026-09-13

Configurable dashboard clock timezone/format and custom web search engines ([#17](https://github.com/boubli/AMUD-Dashboard/issues/17)).

### Added
- **Clock timezone & format** — IANA timezone picker and 12h/24h preference in Settings → Appearance; dashboard clock follows them
- **Custom search engines** — Add HTTPS search URL templates (`{query}` / `%s`) alongside built-ins; choose a default engine

---

## [1.9.2](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.9.2) — 2026-08-12

Docs currency and release hygiene: Theme Gallery / roadmap / README surfaces aligned with the live **41**-theme UI set.

### Changed
- Docs theme definitions and static theme mirror include Crimson Flare; gallery source drops obsolete v1.8.9 themes
- Roadmap Recently shipped / Now / Next / Later refreshed for v1.9.x
- Backfilled `.github/release-notes/v1.9.1.md`; CONTRIBUTING points at `ci-check` + audit/Sonar notes

---

## [1.9.1](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.9.1) — 2026-07-24

New Crimson Flare theme built with UI-UX Pro Max design intelligence, featuring abstract red & orange light flares wallpaper and dynamic light/dark modes.

### Added
- **Crimson Flare Theme** — Vivid crimson light mode & deep obsidian cosmic dark mode with particle flare overlays
- High-resolution wallpaper (`universtock-km9n85Sm9c4-unsplash`), preview thumbnails, and icon pack

---

## [v1.9.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.9.0) — 2026-07-23

Theme shell consolidation (no duplicate index HTML) and SonarCloud maintainability / CPD cleanup.

### Changed
- Single `index.html` for all themes; Glow/Neu chrome via CSS
- SW v50; installer `CURL_SECURE`; Sonar exclusions for docs mirrors

---

## [v1.8.13](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.8.13) — 2026-07-23

Two new advanced themes with dedicated dashboard shells: Glow and Glass, and Neumorphism.

### Added
- Glow and Glass theme (neon cyan glass + light cyan/purple; dedicated shell)
- Neumorphism theme (soft extruded UI; dedicated shell)

---

## [v1.8.12](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.8.12) — 2026-07-22

Post-reboot telemetry: keep last-known containers, CHECKING instead of premature UNKNOWN, softer URL health.

### Fixed
- Agent retains LXC/Docker cache on PVE/Docker fetch failure
- Dashboard badges stay CHECKING while waiting for Proxmox (SW v47)
- URL health needs 2 consecutive failures before OFFLINE

### Docs
- FAQ / troubleshooting note for post-reboot wait

---

## [v1.8.11](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.8.11) — 2026-07-19

Theme wallpaper refresh and Azure Calm gallery rename.

### Changed
- Unsplash wallpapers for five themes; Azure Calm display name for `tokyo-night`; SW v46

---

## [v1.8.10](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.8.10) — 2026-07-16

Settings blank-tab fix and updated product screenshots on README / docs.

### Fixed
- Categories through Audit Log blank Settings panes (extra `</div>` after Performance)

### Changed
- Screenshot gallery: Taghawsa mobile, Add App, Branding; SW v45

---

## [v1.8.9](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.8.9) — 2026-07-15

Settings UX, audit tools, icon gallery, and theme overhaul (five new themes; true Light/Dark variants).

### Added
- Audit / update-history pagination, clear, CSV export; scoped audit API
- Searchable Add/Edit App icon gallery
- Themes: ember-hearth, neon-boulevard, kelp-abyss, amber-console, glacier-mist

### Changed
- Infrastructure rename; Branding absorbs Support; HA via Add App
- Light/Dark as per-theme variants; unique icon packs; SW v44

### Removed
- sunset-warm, vaporwave-grid, ocean-depths, terminal-amber, arctic-frost

---

## [v1.8.1](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.8.1) — 2026-07-13

Jellyfin posters and playback controls, per-app media integration, instant status on reload, and the Taghawsa WebGL theme.

### Added
- **Jellyfin now-playing posters** — real movie/episode artwork on the stream card, proxied through the server so the API key never reaches the browser
- **Playback controls** — admin pause / resume / stop for the active Jellyfin session; visible only while something is streaming
- **Taghawsa theme** — WebGL animated gradient with mouse ripple (38 themes total), static CSS fallback on mobile/reduced-motion
- **Instant status on reload** — SSR-embedded last-known statuses and container CPU/RAM, plus localStorage hydration for statuses and integration metrics

### Changed
- **Media integrations moved to Add App** — Jellyfin/Emby and Plex credentials now live on the app card (Integration dropdown); legacy Settings → Integrations values migrate automatically on first start

### Fixed
- **Container actions** — `HTTP request failed: client error (SendRequest)` resolved with an automatic retry on stale connections and actionable agent error messages
- **App card metrics** are selectable/copyable without opening the app URL

---

## [v1.8.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.8.0) — 2026-07-10

Smart idle runtime, performance presets, multi-node support, and a big RAM efficiency push.

### Added
- **GUI-aware idle/active runtime** — pollers and caches pause when nobody is in the dashboard
- **Performance presets** — Light / Balanced / Active / Custom in Settings
- **Multi-node** — `agent_node_tag` and per-card `node_tag` for multi-host homelabs
- **Batch integrations API** — up to 50 apps per request
- **API token scopes** — selectable scopes in Settings (telemetry, webhooks, integrations, …)
- **Backup reminders** — overdue export banner and reminder interval
- **Idle host alerts** — CPU / RAM / disk webhook thresholds in deep idle
- **Integration wizard** — test endpoint + Custom API templates
- **Paginated apps API** — `GET /api/apps?page=` for large libraries

### Improved
- **Agent** — idle skips unlinked containers, Arc cache reuse, concurrent Docker stats
- **Server** — viewport-capped dashboard (50 cards), stale node eviction, streaming backup export
- **PWA** — offline status cache in localStorage; service worker v36
- **Docs** — i18n locale scaffolding (en, de, fr, es)

---

## [v1.7.7](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.7.7) — 2026-07-09

Optimize without feature loss, per-theme light mode, and RSS feeds toggle.

### Added
- **`feeds_enabled`** — stop background RSS polling and hide `/feeds` while keeping feed sources in SQLite
- **Settings → Performance & resources** — cache TTL/max entries, advanced poll intervals, feeds toggle

### Improved
- **Agent** — selective sysinfo refresh, Docker/LXC poll caches, reused Disks/Components, GPU backoff, configurable intervals via agent config sync
- **Server** — idle poller gating (status/media/HA/coordinator), telemetry broadcast slimming for admin-only sessions, integration cache hot-reload on save
- **Light mode** — per-theme palettes (36 bundled + default); narrowed global light CSS; settings preview uses theme variables
- PWA cache v34

---

## [v1.7.5](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.7.5) — 2026-07-09

Dashboard polish (RAM on cards, settings layout, web search) and server idle memory cuts.

### Added
- **°C / °F** — `weather_temp_unit` in Appearance → Weather
- **Web search engines** — Google, Bing, DuckDuckGo, YouTube, GitHub
- **Git pre-commit hook** — `scripts/setup-githooks.sh` runs `cargo fmt` before commits

### Improved
- **App cards** — container RAM as absolute bytes (`512 MB`, `1.2 GB`), not %
- **Settings** — Weather under Appearance; Proxmox API under Integrations; old tabs auto-migrate
- **Server memory** — telemetry broadcaster gated on WebSocket subscribers; shared HTTP clients for pollers; mimalloc allocator
- **PWA** — service worker cache v32

---

## [v1.7.4](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.7.4) — 2026-07-04

Custom integration picker with logos; Ollama and Open WebUI integrations.

### Added
- **Ollama** — `/api/tags` model count, `/api/ps` running models
- **Open WebUI** — `/api/models` with API key, or `/health` without
- **Integration picker** — custom dropdown with group headers, brand icons, and search filter (replaces native optgroup select)

### Fixed
- **Add/Edit App Integration field** — unreadable white optgroup bars on Windows dark theme

### Improved
- **Manifest API** — `icon` field per integration for picker UI
- **PWA** — service worker cache v31

---

## [v1.7.3](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.7.3) — 2026-07-01

Integration dropdown fix (CSP nonce) and Unraid `su-exec: setgroups` follow-up.

### Fixed
- **Add/Edit App Integration dropdown** — manifest loader script had no CSP nonce; browser blocked it, leaving only "None" ([#16](https://github.com/boubli/AMUD-Dashboard/issues/16) follow-up)
- **Unraid Docker** — dashboard image defaults to UID 99 / GID 100 (no runtime `su-exec`); fixes `setgroups(100): Operation not permitted` under `--cap-drop=ALL`
- **Agent Docker/Compose** — `--user 0` / `user: "0:0"` so agent keeps Docker socket access

### Improved
- **PWA** — service worker cache v30
- **Docs** — troubleshooting for empty Integration list and Unraid setgroups loop

---

## [v1.7.2](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.7.2) — 2026-06-25

Unraid Docker first-boot fix: dashboard runs as PUID 99, clearer permission errors, IPC socket mode.

### Fixed
- **Unraid CA install** — dashboard entrypoint drops to PUID 99 / PGID 100 (matches `nobody:users` appdata); fixes `.amud-secrets-key: Permission denied` on first boot ([#16](https://github.com/boubli/AMUD-Dashboard/issues/16))
- **Agent IPC** — `AMUD_SOCKET_MODE` (default `666` in Docker/Unraid templates) so root agent connects when dashboard is non-root

### Improved
- **Startup errors** — permission-denied hints link to Unraid troubleshooting docs
- **Docker image** — Alpine runtime + `su-exec` entrypoint (agent still runs as root via entrypoint override)

---

## [v1.7.1](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.7.1) — 2026-06-25

Mobile/PWA hotfix: Settings hamburger menu, desktop guest layout restored, shorter admin cards on phones.

### Fixed
- **Settings hamburger** — menu toggle and panel wrapped so outside-click no longer closes immediately
- **Desktop guest cards** — horizontal header layout restored (v1.6.4.1 style); 2-col vertical tiles only on mobile

### Improved
- **Admin app cards (mobile)** — shorter metric tiles, 4-column grid, hidden descriptions, 44px edit controls
- **Service worker** — `sw.js` v29

---

## [v1.7.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.7.0) — 2026-06-25

Integration catalog parity: manifest sync, Homepage import aliases, Watchtower/Ombi/FileBrowser, manifest-driven Add App UI.

### Added
- **Catalog parity** — 80+ types added to `INTEGRATION_CATALOG` (TTL + `/api/integrations/manifest`)
- **Homepage import** — 20+ new `widget.type` aliases (`diskstation`, `cloudflared`, `firefly`, …)
- **Integrations** — Watchtower, Ombi, FileBrowser app card fetchers
- **Manifest-driven UI** — Add/Edit App integration dropdown from server catalog

### Improved
- **Service worker** — `sw.js` v28

---

## [v1.6.5](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.6.5) — 2026-06-25

Mobile PWA follow-ups: guest 2-col grid, weather sizing, admin card headers, Settings hamburger menu.

### Improved
- **Weather card** — `clamp()` font sizing + length-based classes for narrow hero widgets
- **Guest dashboard** — 2-column compact grid on mobile; vertical card layout (icon, name, status badge)
- **Admin app cards** — single-row header on mobile; column layout only for guest compact cards
- **Settings** — hamburger overflow menu replaces horizontal scroll nav on ≤820px
- **Service worker** — `sw.js` v27

---

## [v1.6.4.1](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.6.4.1) — 2026-06-28

Hotfix for v1.6.4 default logo showing raw template syntax instead of AMUD PNG.

### Fixed
- **Default logo** — `apply_app_logo_template` uses span-based `{{if app_logo}}…{{end}}` replacement so empty logo shows CSS default `/static/AMUD-logo.png`
- **Custom logos** — still inject `<img>` when Settings Dashboard Logo is set

---

## [v1.6.4](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.6.4) — 2026-06-28

Branding logo fix for guests/login, PWA mobile polish, custom logo favicon/manifest.

### Fixed
- **Custom logo empty for guests / login** — public `GET /uploads/` for branding images; upload stays admin-only
- **Logo markup** — `<img>` on dashboard, login, settings; favicon and PWA manifest stay in sync
- **Mobile overflow menu** — fixed panel below topbar; outside-click close; no hero overlap

### Improved
- **PWA mobile UI** — centered logo/title, hero layout, weather card, telemetry grid, expandable search
- **Custom logo → PWA** — Settings Dashboard Logo drives favicon, Apple touch icon, manifest
- **Service worker** — `sw.js` v26

### Maintainer
- CI hardening, Docker Hub overview sync, README release list trim

---

## [v1.6.3](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.6.3) — 2026-06-25

ARM64 binaries, multi-arch Docker, GitHub Pages hero + GIFs, PWA/mobile polish.

### Added
- **ARM64 release binaries** — `amud-server-arm64` and `amud-agent-arm64` alongside amd64 builds
- **Multi-arch Docker** — `linux/amd64` and `linux/arm64` images on Docker Hub
- **GitHub Pages deploy** — restored workflow; rotating homepage hero carousel + GIF demo section
- **PWA install UX** — manifest shortcuts, install banner, offline shell (`sw.js` v23)

### Improved
- **Mobile topbar** — overflow actions collapse into a touch-friendly menu at ≤768px
- **`update-amud.sh` + in-app updater** — architecture-aware release asset selection

---

## [v1.6.2](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.6.2) — 2026-06-27

Offline bundled themes — replaces withdrawn v1.6.1.

### Added
- **Manifest v5** — icons, wallpapers, and gallery previews in `ui.tar.gz` (`/static/themes/…`)
- **WebP** wallpapers and preview thumbnails
- **Per-profile icon libraries** (8 art styles across 35 themes)
- **`theme-layouts/`** and extracted **`theme-picker.js`**

### Fixed
- Theme gallery black boxes when CDN unreachable
- Theme save + local wallpaper/icon apply via `theme-engine.js`

### Note
- **v1.6.1 withdrawn** — do not use; upgrade to v1.6.2

---

## [v1.6.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.6.0) — 2026-06-25

Universal dashboard parity — Homepage/Homarr import, integration cache, LDAP, Custom API, long-tail integrations, Plex/Jellyfin cards, *arr calendar, release trackers, multi-node agent.

### Added
- **Integration cache** + poll coordinator for scalable card refresh
- **Homepage YAML** and **Homarr JSON** importers (Settings → Integrations)
- **Homepage Docker labels** in agent discovery
- **Custom API**, **LDAP**, **OIDC admin groups**, **per-user boards**
- **Plex/Jellyfin** per-app cards, **\*arr calendar** widget, **release trackers**
- **40+ integrations** (Autobrr, Gotify, Prometheus, OMV, Kubernetes, …)
- **Server-driven integration manifest** for Add/Edit app UI
- **Multi-node agent** `node_tag` support
- Docs: comparison matrix, migration guides, architecture updates

### Improved
- Generic tier-2 integration card template
- Promoted health-only services to full cards (Gitea, GitLab, Jenkins, MinIO, Kopia, Headscale, Stash, …)

---

## [v1.5.6.4](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.6.4) — 2026-06-25

Theme system fix, 13 new integrations, filled integration cards, no card hover.

### Added
- **Servarr expansion** — Lidarr, Readarr, Whisparr app card integrations
- **Top 10 roadmap integrations** — SABnzbd, NZBGet, Transmission, Jackett, Tautulli, Audiobookshelf, Immich, Tdarr, Maintainerr, Frigate
- **Wallpaper overlay strength** — new slider in Glass & Layout Parameters (0 = clear wallpaper)

### Fixed
- **All 36 bundled themes** — glass opacity, blur, radius, and wallpaper tint respect user sliders (no hardcoded overlays)
- **Integration cards** — 8 metric cells always filled (CPU/RAM show `—` when agent off)
- **App card hover** — removed lift/glow and metric layer swap
- **Guest dashboard** — compact cards no longer leave large empty row gaps (grid rows use `auto` height)

### Improved
- `theme-guards.css` cascade enforcement; theme audit matrix for v1.5.6.4

---

## [v1.5.6.3](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.6.3) — 2026-06-26

Theme gallery overhaul, 37 offline themes, guest compact cards, and docs/CI polish.

### Added
- **37 bundled themes** — visual Theme Gallery in Settings → Appearance; manifest v3 with previews and wallpapers
- **18 new theme packs** — Nature, Terminal, Feminine, Variety
- **Vendored wallpapers** — unique Unsplash/Pexels photos per theme at `/static/themes/wallpapers/`
- **`active_theme_id`** setting for theme picker persistence

### Fixed
- **Guest dashboard** — compact 1×1 cards (icon, name, status only)
- **RSS settings** — add-feed modal; category table column layout
- **CI** — clippy fixes; Docusaurus build in GitHub Actions

### Improved
- Filled integration cards (6-cell grid, richer stats, 30s refresh) — see v1.5.6.2
- GitHub Pages Theme Gallery category filters; `fetch-theme-wallpapers.py`

---

## [v1.5.6.2](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.6.2) — 2026-06-26

Filled integration cards and expanded per-service API stats.

### Fixed
- Integration cards fill tall 1×2 layout with unified 6-cell metrics grid
- Expanded API stats (Radarr, Sonarr, Prowlarr, qBittorrent, Pi-hole, etc.)
- Integration data refreshes every 30s on visible cards

---

## [v1.5.5.6](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.5.6) — 2026-06-25

Dashboard UX and telemetry mapping polish release.

### Fixed
- **App card navigation** — clicking anywhere on a card now opens it, while preserving embedded controls and buttons
- **Integration metrics visibility** — integration stats are now visible by default (no hover dependency)
- **Telemetry mapping normalization** — network interface and disk mount lists are canonicalized and deduplicated
- **Settings guidance** — telemetry field help text clarifies normalization and duplicate-handling behavior

---

## [v1.5.5.5](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.5.5) — 2026-06-24

Hotfix: restore agent telemetry broken in v1.5.5.4.

### Fixed
- **Agent telemetry** — Docker discover handler no longer blocks GPU, host stats, container controls, or container status
- **Status badges** — ONLINE/OFFLINE/RUNNING text restored (latency in tooltip)
- **Search toggle UI**, **accent preview**, **GPU row layout**, service worker cache bump

---

## [v1.5.5.4](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.5.4) — 2026-06-24

Dashboard parity batch: search, layout, widgets, Docker import, OIDC, API tokens, kiosk/share, iframe embeds.

### Added
- **Global search** — app filter + web search toggle; keyboard shortcuts (`Ctrl+K`, `/`, `1`–`9`, `?`)
- **Dashboard layout** — tabs vs collapsible category sections (Settings → Appearance)
- **Status page** — `/status` and `/api/status`
- **Dashboard widgets** — Settings → Widgets CRUD
- **Per-app guest visibility** — hide individual apps from guest sessions
- **Docker discovery** — agent label scan + Settings import UI
- **OIDC SSO** — Security settings + login SSO button
- **API tokens** — Bearer auth on read-only API routes
- **Kiosk mode** + **share links** (`/s/:token`)
- **Iframe embeds** — per-app embed mode, `/embed/:id`, CSP frame-src allowlist

### Fixed
- CI: clippy, cargo audit (oauth2 dep), SonarCloud gate on parity UI code

---

## [v1.5.5.3](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.5.3) — 2026-06-24

Fix broken integration hover from v1.5.5.2; admin settings redirect.

### Fixed
- **Integration hover** — fixed metrics slot swap inside the card (no overlap on neighbors)
- **Admin settings** — redirect to `/login` when not signed in as admin

### Improved (carried from v1.5.5.2)
- Docker `:latest` version display, Docker auto-enable controls, RSS auto-favicons

---

## [v1.5.5.2](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.5.2) — 2026-06-24

**Superseded by v1.5.5.3** — integration hover drawer overlapped other cards. Do not use.


## [v1.5.5.1](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.5.1) — 2026-06-24

qBittorrent and Bazarr integrations plus compact hover stats on app cards.

### Added
- **qBittorrent** and **Bazarr** app card integrations

### Improved
- **Integration stats on hover** — cards stay short; hover to see Prowlarr/*arr/qBit/Bazarr metrics
- **Telemetry mapping docs** — how to find interface names and disk mounts (linked from Settings)

---

## [v1.5.5.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.5.0) — 2026-06-23

Community integrations, telemetry mapping, and Unraid polish (Inch feedback batch).

### Added
- **Prowlarr, Uptime Kuma, Cloudflare Tunnel, Peanut (UPS)** app card integrations
- **Per-app CPU/RAM toggle** on Add/Edit App
- **Host telemetry mapping** — external/internal network interfaces and disk mount points (Settings → Privacy & Access)
- **Guest ONLINE/OFFLINE** container status; **login page branding**

### Improved
- **AdGuard Home** — Basic auth clarity, auto Base64 for raw credentials, correct stats field, safer widget fetch
- **Unraid docs** — permission fix and Docker password reset
- **GitHub Issues** guidance for bugs and features

---

## [v1.5.4.3](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.4.3) — 2026-06-23

Hardening fix when live telemetry still failed after v1.5.4.2 (cached HTML / service worker).

### Fixed
- **Live dashboard scripts** — clock, WebSocket, GPU card, telemetry bars, and status badges moved to versioned `/static/dashboard-live.js`
- **Service worker** — do not cache HTML navigations; cache bump clears stale dashboard JS

---

## [v1.5.4.2](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.4.2) — 2026-06-23

Critical fix: live telemetry and status badges after feeds UI refactor.

### Fixed
- **Dashboard JavaScript crash** in `updateClock()` — WebSocket, GPU card, CPU/RAM bars, and CHECKING badges work again

---

## [v1.5.4.1](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.4.1) — 2026-06-23

Audit log migration fix for upgraded databases.

### Fixed
- **Audit log schema** — auto-adds missing `username` column on startup; backfills from legacy `user` column when present

---

## [v1.5.4.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.4.0) — 2026-06-23

Feeds page polish: hero headline, category colors, and drag reorder.

### Added
- **Featured headline hero** on `/feeds` — top story from the first feed in sort order
- **RSS feed reorder** — drag rows in Settings → RSS Feeds; order drives `/feeds` layout

### Improved
- **Feed category tabs** — guest-visible accent colors per category on `/feeds`
- **Feeds UX** — news-style cards, preset icons, and category empty states (phases 1–3)

---

## [v1.5.3.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.3.0) — 2026-06-23

RSS management UI, homelab polish, and audit expansion.

### Added
- **RSS Feeds settings tab** — add, edit, and delete RSS/Atom feeds from Settings
- **Dedicated `/feeds` page** — RSS-only view with nav link for all visitors
- **RSS in Add/Edit App modal** — create feed cards from the dashboard
- **Webhook private LAN toggle** — allow homelab webhooks to `192.168.x.x` (Settings → Privacy)
- **Backup validate endpoint** — preview app/user/webhook counts before restore
- **Audit log filter** — search and filter by action in Settings
- **Webhook quick presets** — Discord, Telegram, and generic JSON shortcuts

### Improved
- **Audit logging** — wake-on-LAN, card reorder, and category CRUD now recorded
- **Backup tab** — last export timestamp, secrets key reminder, import confirmation with counts
- **Home Assistant TLS** — respects “Accept invalid TLS certificates” setting
- **Docs** — full features page, updated roadmap and homepage

### Fixed
- Guest users can discover the Feeds page from the topbar
- Feeds page shows a clear empty state when no RSS apps exist

---

## [v1.5.2.1](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.2.1) — 2026-06-23

Overseerr/Jellyseerr integrations, multi-language READMEs, and Docker build optimizations.

### Added
- **Overseerr & Jellyseerr Integrations** — Native API support to display live pending media request counts on dashboard cards.
- **Multi-Language Support** — README documentation in Arabic, Hindi, Spanish, French, German, Italian, Portuguese, Russian, Chinese, Japanese, and Korean.

### Improved
- **Docker Build Optimizations** — Overhauled `.dockerignore` to reduce image size and build times.

---

## [v1.5.2.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.2.0) — 2026-06-23

Native RSS and Atom feed integration for dashboard app cards.

### Added
- **RSS Integration**: Add any valid RSS or Atom feed URL to an app to display the top 3 latest headlines on its card.
- **Guest Visibility**: RSS integration data is explicitly permitted for unauthenticated guest users, keeping your dashboard lively while keeping sensitive integrations (like Pi-hole or Proxmox) locked down to admins.
- **Feed Parsing**: Powered by `feed-rs` for robust cross-format feed compatibility.

---

## [v1.5.1.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.1.0) — 2026-06-21

Appearance cleanup, offline bundled themes, audit log fixes, and in-app updater repair.

### Added
- Bundled theme picker in Settings (18 CSS files + manifest, works offline)
- Six new advanced themes: Terminal Phosphor, Vaporwave Grid, Blueprint Tech, Luxury Gold, Holographic Prism, Brutalist Mono
- Theme gallery **Download CSS** button; per-file theme definitions in docs
- Pre-commit `cargo fmt` hook, `check-rust` scripts, and `rustfmt.toml`

### Improved
- Appearance tab simplified — overlay tint presets removed; quick colors + sliders + Custom CSS
- Live preview scoped to mini scene; Custom CSS updates in real time
- Audit log schema on startup, 503 on read failure, settings-save audit entry
- Troubleshooting docs for audit log

### Fixed
- Proxmox in-app updater SHA256 lookup (basename matching in `SHA256SUMS`)
- Release workflow checksum layout aligned with updater
- CI rustfmt drift; Sonar theme re-export

See [release notes](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.1.0).

---

## [v1.5.0.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.0.0) — 2026-06-20

Major visual upgrade, drag-and-drop layout, light mode, and docs expansion.

### Added
- Cinematic dashboard animations (staggered cards, hover effects, ambient orbs, status pulse, greeting shimmer)
- Admin drag-and-drop card reorder with CSRF-protected API and SQLite `sort_order`
- Bento card spans (`1x1`, `2x1`, `1x2`) with mobile collapse
- Light mode theme (`data-theme="light"`) across dashboard and settings
- Video wallpaper support (`.mp4`, `.webm`, `.ogg`)
- Live settings preview (accent, glass, wallpaper, overlay, grid columns, accent glow)
- Docusaurus blog, FAQ, `llms.txt`, theme gallery, and JSON-LD SEO

### Improved
- Handle-only drag with category-filter guard, error toasts, rollback, and touch support
- Settings wallpaper layers and theme mode on page load (no flash)
- Light mode background polish and contrast fixes
- Docker CI resilience; Sonar maintainability fixes

### Fixed
- New apps get `MAX(sort_order)+1`; reorder validates full app set
- Add App modal preserves `card_span: '1x1'`
- Admin-only `drag.js` loading

See [release notes](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.5.0.0).

---

## [v1.4.2.2](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.4.2.2) — 2026-06-20

Host telemetry and status polish on the main dashboard only.

### Removed
- Dedicated `/telemetry` page and **System** topbar button

### Fixed
- Host-based apps (e.g. Beszel, Filebrowser) now map correctly even when they are not LXC containers
- Proxmox card status now follows host agent connectivity (no false OFFLINE from URL check)
- Dashboard app matching improved via normalized aliases and URL-derived tokens

### Improved
- Dashboard top telemetry row shows CPU, RAM, GPU (when available), Disk, and Bandwidth
- Guest (not logged in) app cards render in compact mode to avoid oversized empty cards

---

## [v1.4.1.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.4.1.0) — 2026-06-20

Proxmox card, host telemetry (CPU model + GPU), and settings fixes. See [release notes](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.4.1.0).

### Added
- Host CPU model, cores, temperature, and live CPU/Memory sparklines
- GPU telemetry via `nvidia-smi` on the Proxmox host agent
- **Privacy & Access** settings tab (guest telemetry + TLS)

### Fixed
- Proxmox false OFFLINE when agent is connected
- Per-app container CPU/RAM on dashboard cards
- WebSocket telemetry updates stopping on null DOM nodes
- Settings logo field showing `{{app_logo}}` placeholder

### Changed
- Proxmox Jellyfin-style stream card in app category section
- Guest/TLS settings moved out of Support / Donation

---

## [v1.4.0.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.4.0.0) — 2026-06-20

Security, audit log, and UI overhaul. See [release notes](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.4.0.0) for full details.

### Added
- Audit log (SQLite + Settings tab)
- `.env.example` for security-related env vars

### Changed
- SQLite WAL + foreign keys
- Dashboard status badges and Proxmox-style chips
- Settings DOM rendering via `admin.js` helpers
- SonarCloud quality gate passing; Actions pinned to SHAs

### Security
- Webhook SSRF filtering and URL masking
- HTML escaping on branding fields in templates

---

## [v1.3.7.3](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.3.7.3) — 2026-06-20

Settings sidebar scroll fix on smaller screens.

## [v1.3.7.2](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.3.7.2) — 2026-06-20

Auto-updater fix; CI runner ubuntu-22.04 for older LXC GLIBC.

## [v1.3.7.1](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.3.7.1) — 2026-06-20

Auto-update system and version notifications.

## [v1.3.7](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.3.7) — 2026-06-20

Wake-on-LAN decoupling; Proxmox UDS permission fixes.

## [v1.3.6](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.3.6) — 2026-06-18

Stability and deployment improvements.

## [v1.3.1.5](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.3.1.5) — 2026-06-14

Maintenance release.

## [v1.3.0.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.3.0.0) — 2026-06-10

Feature release.

## [v1.2.0.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.2.0.0) — 2026-06-09

Architecture and telemetry updates.

## [v1.1.0.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.1.0.0) — 2026-06-06

Fully static architecture and security hardening.

## [v1.0.0](https://github.com/boubli/AMUD-Dashboard/releases/tag/v1.0.0) — 2026-06-03

Initial public release.
