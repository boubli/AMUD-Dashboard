# Mirror .github/workflows/ci.yml locally before push (Windows).
# Usage: .\scripts\ci-check.ps1
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

Write-Host "==> cargo fmt (auto-fix)"
cargo fmt --all
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> cargo fmt --check"
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> cargo test (lib)"
cargo test --workspace --lib
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if ((Test-Path "docs") -and (Test-Path "docs/package-lock.json")) {
    Write-Host "==> docs build"
    Push-Location docs
    try {
        npm ci
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        npm run build
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    } finally {
        Pop-Location
    }
}

Write-Host "All CI checks passed."
