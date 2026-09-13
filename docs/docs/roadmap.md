---
sidebar_position: 3
title: Roadmap
---

# Roadmap

This is where AMUD is headed. No dates — homelab project, ship when it's ready.

Items move between **Now**, **Next**, and **Later** as reality hits. Recently shipped work lives in the [Changelog](./changelog).

---

## Recently shipped (v1.9.x / late v1.8.x)

- **v1.9.3** — Clock timezone/format + custom web search engines ([#17](https://github.com/boubli/AMUD-Dashboard/issues/17))
- **v1.9.2** — Docs currency: Theme Gallery + roadmap + README surfaces synced to **41** themes; release-notes hygiene
- **v1.9.1** — **Crimson Flare** theme (crimson light / obsidian dark, flare wallpaper, icon pack)
- **v1.9.0** — Single dashboard HTML shell for Glow/Neu; SonarCloud maintainability + CPD exclusions
- **v1.8.13** — **Glow and Glass** + **Neumorphism** themes
- **v1.8.12** — Post-reboot telemetry: keep LXC/Docker cache on PVE failure; CHECKING not premature UNKNOWN; softer URL health
- **v1.8.9–v1.8.11** — Theme replacements, mobile/performance polish, Taghawsa cross-device WebGL

Older releases (v1.8.0–v1.8.8, v1.7.x, v1.6.x, …): see the full [Changelog](./changelog).

---

## Now (active focus)

- **Quality bar** — keep SonarCloud green, `cargo audit` clean, CI integration tests
- **Docs ↔ UI lockstep** — theme gallery definitions, docs static theme mirror, and version surfaces stay aligned with the shipped UI set
- **Net-new Homepage widgets** — watchtower/ombi/filebrowser batch shipped in v1.7.0; continue small batches (ombi-adjacent, file tools) or Custom API templates

---

## Next

- **Docusaurus locale packs** — translate docs site (READMEs already in 11 languages)
- **Backup/restore UX** — export *scheduling* reminders (overdue export banner already shipped)

---

## Later (ideas, not commitments)

- **Multi-node agent UI** — per-app `node_tag` shipped in v1.6.0; later = aggregate telemetry from several agents in one dashboard view
- **API tokens** — selectable scopes already ship when creating tokens; expand further for telemetry/feeds/webhooks as needed
- **Per-integration setup wizards** — guided Pi-hole, *arr, and DNS blocker configuration in Settings
- **PWA offline polish** — richer offline shell and cache strategy beyond static asset precache

---

## Won't do (on purpose)

- **YAML as primary config** — SQLite + UI is the whole point.
- **Node.js/PHP runtime dependency** — compiled Rust only.
- **SaaS / cloud-hosted AMUD** — self-hosted homelab tool, stays that way.

---

## Request a feature

Open a [GitHub Issue](https://github.com/boubli/AMUD-Dashboard/issues) with the `enhancement` label for **bugs and feature requests** (preferred — tracked per release). Use [Discussions](https://github.com/boubli/AMUD-Dashboard/discussions) for questions and screenshots. If it fits the homelab/self-hosted scope, it lands here.

**Shipped something?** It's in the [Changelog](./changelog).
