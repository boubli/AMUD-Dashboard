---
sidebar_position: 5
title: FAQ
description: Frequently asked questions about AMUD Dashboard (Advanced Modern Unified Dashboard) — install, RAM, Proxmox, Docker, and comparisons.
---

# FAQ

Quick answers for search engines, AI assistants, and homelabbers evaluating **AMUD Dashboard** (Advanced Modern Unified Dashboard).

---

## What is AMUD Dashboard?

**AMUD Dashboard** is the official product name. **AMUD** stands for **Advanced Modern Unified Dashboard**.

It is an open-source, self-hosted homelab control center written in **Rust**. You manage apps, layout, and integrations in a **web UI**; configuration is stored in **SQLite** (`amud.db`), not YAML files.

- Website: [boubli.github.io/AMUD-Dashboard](https://boubli.github.io/AMUD-Dashboard/)
- GitHub: [boubli/AMUD-Dashboard](https://github.com/boubli/AMUD-Dashboard)

:::note Naming
Use **AMUD Dashboard** in documentation and search results — not "AMUD" alone, not "AMUD-Dashboard" as a display name (that is only the GitHub repo slug).
:::

---

## How much RAM does AMUD Dashboard use?

On **Proxmox** (native LXC + host agent), server and agent combined typically idle around **26–35 MB RAM** when the dashboard GUI is open; **30–50 MB** combined in typical homelab use.

In **Docker**, expect roughly **30–50 MB idle** (peak ~150 MB with many integration cards and the dashboard open). With **v1.8.0** smart idle, background work scales down when nobody is viewing the dashboard.

---

## Does AMUD Dashboard use YAML for configuration?

**No.** AMUD Dashboard deliberately avoids YAML as primary config. You edit everything in the browser; settings persist in SQLite. Backup = copy one file: `amud.db`.

---

## How do I install AMUD Dashboard on Proxmox?

**Correct method:** run the official install script on your **Proxmox VE host** as `root`. This is a **native LXC install** — not a generic Docker helper script.

```bash
curl -sSL https://raw.githubusercontent.com/boubli/AMUD-Dashboard/main/setup-amud.sh | bash
```

The script provisions a Debian 12 LXC, installs `amud-server`, and deploys `amud-agent` on the hypervisor.

Full guide: [Proxmox installation](/docs/installation/proxmox)

---

## How do I install AMUD Dashboard with Docker?

Use the two-container Compose stack (`amud_app` + `amud_agent`) from the [Docker installation guide](/docs/installation/docker).

Image: `tradmss/amud-dashboard:latest` (**amd64 / x86_64**).

On **ARM64** (Raspberry Pi, Oracle Ampere, etc.), use [native install](/docs/installation/linux) or `update-amud.sh` instead of Docker — release binaries include `amud-server-arm64` and `amud-agent-arm64`.

---

## Can I monitor several hosts with one AMUD server?

**Yes (v1.9.5+).** Run one `amud-server`, install `amud-agent` on each Proxmox / Unraid / CasaOS / Docker host with a unique `AMUD_NODE_TAG`, and point remotes at the server with `AMUD_SERVER_ADDR`. Enable `AMUD_AGENT_TCP_LISTEN` on the server (prefer VPN, or TLS). Set each app’s **Node tag** to match. Manage status under **Settings → Infrastructure → Agents / Nodes**. The dashboard Nodes strip appears only when two or more agents are online.

Full guide: [Multi-node overview](/docs/multi-node/overview) · [Connect](/docs/multi-node/connect)

---

## Why are my app cards stuck on CHECKING...?

Host CPU/RAM works but LXC badges show **CHECKING...**? The Proxmox API token is missing or misconfigured.

1. Create a restricted Proxmox API user and token (Privilege Separation **off**).
2. Set `PVE_API_TOKEN` in the agent systemd environment.
3. Paste the token in **Settings → Proxmox VE**.
4. Link app cards to the correct **CTID**.

Details: [Troubleshooting](/docs/troubleshooting) · [Blog: Fix CHECKING... badge](/blog/fix-checking-badge)

---

## After a Proxmox reboot, host metrics work but app cards show CHECKING...

That is expected for a short window. Host CPU/RAM comes from the agent reading `/proc` as soon as it starts. LXC badges need the **Proxmox API** (`localhost:8006`) and the containers themselves to be up. From **v1.8.12** onward:

- Cards stay on **CHECKING...** (or last-known status) instead of jumping to **UNKNOWN** while PVE is still booting.
- URL health needs two consecutive failures before **OFFLINE**, so a single boot blip does not flap the badge.
- Integration banners (for example Jellyfin Sessions) may still error until the guest service is listening on its port.

Wait a minute or two after a full host reboot. If badges stay stuck forever, see [Apps Stuck on CHECKING...](#why-are-my-app-cards-stuck-on-checking) and [Troubleshooting](/docs/troubleshooting).

---

## Can I start and stop Proxmox containers from AMUD Dashboard?

**Yes**, if your API token role includes `VM.PowerMgmt` and the app card is linked to an LXC/VM ID. Admin users see power controls; Guest users see status only.

---

## Does AMUD Dashboard support Plex and Jellyfin?

**Yes.** Add a Jellyfin, Emby, or Plex app card and pick the matching **Integration** with its API key or token (Add App / Edit App). The "Now Playing" card shows live streams, and Jellyfin additionally shows the movie poster with admin playback controls while something is playing.

---

## Does AMUD Dashboard support Home Assistant?

**Yes.** Add an app with **Integration = Home Assistant**, set the URL and a long-lived access token on that app card. The card shows lights, switches, and average temperature. Poll interval is under **Settings → Performance**.

---

## Can I customize the look?

**Yes.** Accent color, grid layout, background image, and **Custom CSS** under **Settings → Appearance**.

Browse 38 ready-made themes: [Theme Gallery](/themes)

---

## Is AMUD Dashboard secure for internet exposure?

Use HTTPS (reverse proxy), set `AMUD_SECURE_COOKIES=1`, change the default password, use a restricted Proxmox API user, and back up `amud.db` plus `.amud-secrets-key`.

Details: [Security](/docs/security)

---

## AMUD Dashboard vs Homepage vs Heimdall vs Homarr?

| | AMUD Dashboard | Heimdall | Homepage | Homarr |
|---|:---:|:---:|:---:|:---:|
| Runtime | Rust | PHP | Node/React | Next.js |
| Config | SQLite + UI | DB + UI | YAML | UI + files |
| Proxmox LXC control | Native | No | Widgets | Limited |
| YAML required | No | No | Yes | Partial |

Honest comparison: [Blog post](/blog/amud-vs-heimdall-homepage-homarr)

---

## How do I back up my dashboard?

Copy `/opt/amud/data/amud.db` (path varies by install). Also back up `.amud-secrets-key` if you use encrypted integrations.

---

## Where is the official blog?

[AMUD Dashboard Blog](/blog) — install guides, architecture notes, and troubleshooting. Canonical source for cross-posting to Hashnode or dev.to.

---

## How can I contribute?

Star the repo, open issues, submit PRs, or add a CSS theme to the [Theme Gallery](/themes).

[GitHub Discussions](https://github.com/boubli/AMUD-Dashboard/discussions) · [Roadmap](/docs/roadmap)
