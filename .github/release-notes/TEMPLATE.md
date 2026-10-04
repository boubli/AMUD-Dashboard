## vX.Y.Z — Short title

One-line summary of the release.

### Added
- New features

### Changed
- Behavior or UI changes

### Fixed
- Specific regressions or defects resolved in this version

### Upgrade

**Native / Proxmox**

```bash
curl -sSL https://github.com/boubli/AMUD-Dashboard/releases/download/vX.Y.Z/update-amud.sh | bash
```

Or, if you already have the updater locally: `./update-amud.sh`

**Docker**

```bash
docker compose pull && docker compose up -d --force-recreate
```

Image: `tradmss/amud-dashboard:latest` (or pin `tradmss/amud-dashboard:vX.Y.Z`)

**Unraid**

1. Open **Apps** → installed **AMUD-Dashboard** (and **AMUD-Agent** if used).
2. **Check for Updates** → apply / recreate the container(s).
3. Hard-refresh the browser (Ctrl+Shift+R). For remote agents, keep `network_mode: host` and matching `AMUD_AGENT_SECRET` / `AMUD_NODE_TAG`.

Hard-refresh your browser after updating if you use the PWA.

**Recommended:** `vX.Y.Z`

**Binaries:** `amud-server`, `amud-agent` (amd64 + arm64), `ui.tar.gz`, and `SHA256SUMS` are attached below.
