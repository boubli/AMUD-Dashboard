---
sidebar_position: 3
title: TCP and TLS
---

# TCP and TLS for remote agents

Local co-located agents keep using a **Unix domain socket**. Remote agents use **TCP**. You choose whether that TCP is plain (VPN/LAN) or TLS-wrapped.

## Option A — Plain TCP over VPN (recommended for most labs)

Use Tailscale, WireGuard, or a trusted LAN.

**Server**

```bash
AMUD_AGENT_TCP_LISTEN=0.0.0.0:8050
AMUD_AGENT_SECRET=your-long-shared-secret
# AMUD_AGENT_TLS unset or 0
```

**Agent**

```bash
AMUD_SERVER_ADDR=100.x.y.z:8050   # or MagicDNS / LAN hostname
AMUD_NODE_TAG=unraid-1
AMUD_AGENT_SECRET=your-long-shared-secret
```

**Do not** expose plain TCP port 8050 on the public internet.

## Option B — TLS

Encrypt agent IPC when you cannot rely on a VPN mesh.

**Server**

```bash
AMUD_AGENT_TCP_LISTEN=0.0.0.0:8050
AMUD_AGENT_TLS=1
AMUD_AGENT_TLS_CERT=/opt/amud/certs/agent.crt
AMUD_AGENT_TLS_KEY=/opt/amud/certs/agent.key
AMUD_AGENT_SECRET=your-long-shared-secret
```

Generate a self-signed cert (example):

```bash
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout agent.key -out agent.crt -days 825 \
  -subj "/CN=amud.lan"
```

**Agent**

```bash
AMUD_SERVER_ADDR=amud.lan:8050
AMUD_NODE_TAG=pve-2
AMUD_AGENT_SECRET=your-long-shared-secret
AMUD_AGENT_TLS=1
AMUD_AGENT_TLS_CA=/path/to/agent.crt   # or your CA PEM
```

Lab-only escape hatch (discouraged): `AMUD_AGENT_TLS_INSECURE=1` skips certificate verification.

## Firewall checklist

1. Server listens on 8050 (or your chosen port).
2. Only VPN/LAN peers can reach it.
3. Agents use the same port in `AMUD_SERVER_ADDR`.
4. Secret matches on every process.

## Auth reminder

IPC still uses challenge–response with `AMUD_AGENT_SECRET`. TLS protects the transport; the secret still authenticates the agent.
