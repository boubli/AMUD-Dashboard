---
sidebar_position: 6
title: Troubleshooting
---

# Multi-node troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `AMUD_NODE_TAG is required` | Remote mode without a tag | Set a unique `AMUD_NODE_TAG` with `AMUD_SERVER_ADDR` |
| `node_tag already connected` | Two agents share one tag | Rename one tag and restart that agent |
| Node never appears in Settings | Secret, firewall, or server not listening | Match `AMUD_AGENT_SECRET`; set `AMUD_AGENT_TCP_LISTEN`; open 8050 on VPN/LAN only |
| Nodes strip missing with 2 hosts | Only one agent actually online | Check Agents / Nodes table; second agent not authenticated |
| Metrics OK, controls fail | App Node tag ≠ agent tag | Edit the app and set the correct tag |
| TLS handshake fails | CA mismatch | Point `AMUD_AGENT_TLS_CA` at the server cert/CA; avoid `INSECURE` except in lab |
| Wrong host on CPU cards | Selected node chip | Click the correct chip on the Nodes strip (multi-host only) |
| Discover hits the wrong box | Default session is Local / first agent | Run discover from the host that has Docker, or use that node’s agent |

## Still stuck?

- [Connect a node](./connect)
- [TCP and TLS](./tcp-and-tls)
- [FAQ — several hosts](/docs/faq#can-i-monitor-several-hosts-with-one-amud-server)
- GitHub Issues with the `enhancement` / bug labels
