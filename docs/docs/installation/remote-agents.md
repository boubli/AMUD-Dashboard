---
sidebar_position: 8
title: Remote agents / multi-node
---

# Remote agents / multi-node

AMUD supports **one server** and **many agents** (Proxmox, Unraid, CasaOS, Docker hosts).

Full documentation (recommended):

- [Overview](/docs/multi-node/overview) — architecture and when to use multi-node
- [Add a second host (agent only)](/docs/multi-node/connect) — Node 1 dashboard → Node 2 agent-only (Docker / Unraid / native)
- [TCP and TLS](/docs/multi-node/tcp-and-tls) — VPN vs encrypted IPC
- [Platforms](/docs/multi-node/platforms) — Proxmox / Unraid / CasaOS / Docker recipes
- [Dashboard controls](/docs/multi-node/controls) — Nodes strip, app tags, Settings panel
- [Troubleshooting](/docs/multi-node/troubleshooting)

**In the app:** Settings → Infrastructure → Agents / Nodes (live status + copy-paste env).

The dashboard **Nodes** strip appears only when two or more agents are connected.
