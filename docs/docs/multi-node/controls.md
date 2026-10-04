---
sidebar_position: 5
title: Dashboard controls
---

# Dashboard and Settings controls

## Nodes strip (dashboard)

- **Hidden** when fewer than two agents report telemetry.
- **Visible** with two or more agents: click a chip to drive the host CPU/RAM/GPU/disk/bandwidth cards.
- Selection is remembered in `localStorage` (`amud_selected_node`).

Single-host installs keep the familiar telemetry row with no extra chrome.

## App Node tag

Every app card can set **Node tag** (Add/Edit App):

- Must match an agent’s `AMUD_NODE_TAG` (or `Local` for the co-located agent).
- Live container status and metrics come from that node’s telemetry.
- Start / stop / restart / reboot / shutdown are sent only to that agent’s session.

If the tag’s agent is offline, the API returns a clear “Agent for node '…' is not connected” error.

## Settings → Infrastructure → Agents / Nodes

| Panel area | Purpose |
|---|---|
| Default local agent node tag | Server setting pushed when the local agent has no env override |
| Live table | Tag, online/offline, platform, capabilities, CPU/RAM, last seen |
| VPN / TLS cards | Copy server listen env |
| Remote agent snippet | Copy per-host agent env |
| Doc links | Jump into this documentation set |

Refresh polls `/api/telemetry` (includes `nodes` + `nodes_meta`).

## Discover / Proxmox test

Docker discover and Proxmox test prefer the **Local** agent session when present, otherwise the first connected agent. Pass a `node_tag` form field when you need a specific remote (advanced).

## Guest visibility

Guests do not receive the full `nodes` map unless guest telemetry is enabled. Multi-node management stays an admin Settings concern.
