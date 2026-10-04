---
sidebar_position: 2
title: Connect a node
---

# Connect a remote agent

This walkthrough adds a **second host** to an existing AMUD install. The first agent (usually tag `Local` over UDS) can stay as-is.

## Prerequisites

- Working AMUD dashboard (any install method)
- Shared secret already used by the local agent (`AMUD_AGENT_SECRET`)
- Network path from the remote host to the server (LAN or VPN)

## 1. Enable TCP on the server

Set on the **dashboard / amud-server** process (and publish the port in Docker):

```bash
AMUD_AGENT_TCP_LISTEN=0.0.0.0:8050
AMUD_AGENT_SECRET=your-long-shared-secret
```

Firewall: allow **8050/tcp** only from LAN or VPN.

In the UI you can copy the same block from **Settings → Infrastructure → Agents / Nodes**.

## 2. Install agent-only on the remote host

Do **not** run a second `amud-server`.

```bash
AMUD_SERVER_ADDR=amud.lan:8050
AMUD_NODE_TAG=unraid-1          # REQUIRED — unique per host
AMUD_AGENT_SECRET=your-long-shared-secret
AMUD_PLATFORM=unraid            # optional badge
```

Mount `/var/run/docker.sock` for Docker controls. On Proxmox, also set `PVE_API_TOKEN` (agent talks to `localhost:8006` on that node).

## 3. Wire apps

1. Open the dashboard — with two agents online, the **Nodes** strip appears.
2. Edit each app → **Node tag** = the agent that runs that container.
3. Start/stop on those cards targets that agent only.

## 4. Verify in Settings

Open **Settings → Infrastructure → Agents / Nodes**:

- Both tags show **Online**
- Platform / capabilities look correct
- Refresh updates CPU/RAM

## Environment quick reference

### Server

| Variable | Purpose |
|---|---|
| `AMUD_AGENT_SECRET` | Shared IPC secret |
| `AMUD_SOCKET_PATH` | Local UDS (co-located agent) |
| `AMUD_AGENT_TCP_LISTEN` | e.g. `0.0.0.0:8050` for remotes |
| `AMUD_AGENT_TLS` / `CERT` / `KEY` | Optional TLS — see [TCP & TLS](./tcp-and-tls) |

### Agent

| Variable | Purpose |
|---|---|
| `AMUD_SERVER_ADDR` | `host:port` of the server (enables remote mode) |
| `AMUD_NODE_TAG` | Unique id (**required** with `AMUD_SERVER_ADDR`) |
| `AMUD_PLATFORM` | Cosmetic: `proxmox` / `unraid` / `casaos` / `docker` |
| `AMUD_DOCKER` | `0` to disable Docker; otherwise auto if socket exists |
| `PVE_API_TOKEN` / `PVE_NODE` | Proxmox on that host |

## Docker Compose — remote agent only

```yaml
services:
  amud-agent:
    image: tradmss/amud-agent:latest
    environment:
      AMUD_SERVER_ADDR: "amud.lan:8050"
      AMUD_NODE_TAG: "docker-box"
      AMUD_AGENT_SECRET: "your-long-shared-secret"
      AMUD_PLATFORM: "docker"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    restart: unless-stopped
```

See also: [Platforms](./platforms) · [Controls](./controls)
