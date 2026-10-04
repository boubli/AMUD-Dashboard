---
sidebar_position: 4
title: Platforms
---

# Platform recipes

AMUD does **not** call Unraid or CasaOS proprietary APIs in this release. Every platform uses the same agent with **capability profiles**.

## Proxmox

Install `amud-agent` **on each PVE host** you want LXC controls for.

```bash
AMUD_SERVER_ADDR=amud.lan:8050
AMUD_NODE_TAG=pve-node1
AMUD_AGENT_SECRET=...
AMUD_PLATFORM=proxmox
PVE_API_TOKEN=user@pam!tokenid=secret
# PVE_NODE=pve   # optional; defaults to hostname
```

The agent talks to `https://localhost:8006` on **that** host. Multi-Proxmox = multiple agents, unique tags.

## Unraid

Agent container with Docker socket + unique tag:

```bash
AMUD_SERVER_ADDR=amud.lan:8050
AMUD_NODE_TAG=unraid-1
AMUD_AGENT_SECRET=...
AMUD_PLATFORM=unraid
```

Mount `/var/run/docker.sock`. For disk mapping, see [Configuration — host telemetry](/docs/configuration#host-telemetry-mapping) (`/mnt/user`, `/mnt/cache`).

## CasaOS

Same as Docker-only; set the cosmetic platform label:

```bash
AMUD_PLATFORM=casaos
# + Docker socket + AMUD_SERVER_ADDR + AMUD_NODE_TAG
```

## Docker-only host / VPS

```bash
AMUD_SERVER_ADDR=amud.lan:8050
AMUD_NODE_TAG=docker-box
AMUD_AGENT_SECRET=...
AMUD_PLATFORM=docker
```

```yaml
volumes:
  - /var/run/docker.sock:/var/run/docker.sock
```

## What each capability unlocks

| Profile | Host metrics | Container list | Power controls |
|---|---|---|---|
| `host` | Yes | — | — |
| `docker` | Yes | Docker | start/stop/restart |
| `proxmox` | Yes | LXC | start/stop/reboot/shutdown |

See [Connect](./connect) for the shared wiring steps.
