#!/usr/bin/env python3
"""Generate shared theme icon style libraries and rewrite per-theme pack.json files."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "ui" / "static" / "themes" / "icons"
STYLES = ICONS / "_styles"

# Logical Lucide path data (inner markup only) for outline-family icons.
PATHS: dict[str, str] = {
    "sun": '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"/>',
    "moon": '<path d="M20 15a7 7 0 1 1-9-9 6 6 0 0 0 9 9z"/>',
    "cloud": '<path d="M17.5 19a4.5 4.5 0 1 0-1.2-8.84A6 6 0 1 0 6 17.5"/>',
    "cpu": '<rect x="5" y="5" width="14" height="14" rx="2"/><path d="M9 9h6v6H9zM9 1v3M15 1v3M9 20v3M15 20v3M1 9h3M1 15h3M20 9h3M20 15h3"/>',
    "hard-drive": '<path d="M22 12H2M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><circle cx="6" cy="16" r="1"/><circle cx="10" cy="16" r="1"/>',
    "activity": '<path d="M22 12h-4l-3 9L9 3l-3 9H2"/>',
    "wifi": '<path d="M5 12.55a11 11 0 0 1 14.08 0M1.42 9a16 16 0 0 1 21.16 0M8.53 16.11a6 6 0 0 1 6.95 0"/><circle cx="12" cy="20" r="1"/>',
    "settings": '<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>',
    "layout-grid": '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/>',
    "layout-template": '<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18M9 21V9"/>',
    "search": '<circle cx="11" cy="11" r="7"/><path d="m21 21-4.3-4.3"/>',
    "plus": '<path d="M12 5v14M5 12h14"/>',
    "bell": '<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/>',
    "users": '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"/>',
    "rss": '<path d="M4 11a9 9 0 0 1 9 9M4 4a16 16 0 0 1 16 16"/><circle cx="5" cy="19" r="1"/>',
    "server": '<rect x="3" y="4" width="18" height="6" rx="1"/><rect x="3" y="14" width="18" height="6" rx="1"/><path d="M7 7h.01M7 17h.01"/>',
    "plug": '<path d="M12 22v-5M9 8V2M15 8V2M6 8h12v4a6 6 0 0 1-12 0z"/>',
    "home": '<path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M9 22V12h6v10"/>',
    "shield": '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10"/>',
    "shield-check": '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10"/><path d="m9 12 2 2 4-4"/>',
    "database": '<ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5v14a9 3 0 0 0 18 0V5"/><path d="M3 12a9 3 0 0 0 18 0"/>',
    "zap": '<path d="M13 2 3 14h9l-1 8 10-12h-9l1-8z"/>',
    "eye": '<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7z"/><circle cx="12" cy="12" r="3"/>',
    "palette": '<circle cx="13.5" cy="6.5" r="1.5"/><circle cx="17.5" cy="10.5" r="1.5"/><circle cx="8.5" cy="7.5" r="1.5"/><circle cx="6.5" cy="12.5" r="1.5"/><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.93 0 1.5-.67 1.5-1.5 0-.39-.15-.74-.39-1.01-.23-.26-.38-.61-.38-.99 0-.83.67-1.5 1.5-1.5H16c3.31 0 6-2.69 6-6 0-4.96-4.49-9-10-9z"/>',
    "arrow-left": '<path d="m12 19-7-7 7-7M19 12H5"/>',
    "external-link": '<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><path d="M15 3h6v6M10 14 21 3"/>',
    "power": '<path d="M12 2v10M18.4 6.6a9 9 0 1 1-12.77.04"/>',
    "play": '<path d="m6 3 14 9-14 9V3z"/>',
    "pause": '<rect x="6" y="4" width="4" height="16" rx="1"/><rect x="14" y="4" width="4" height="16" rx="1"/>',
    "refresh": '<path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/><path d="M8 16H3v5"/>',
    "cloud-sun": '<path d="M12 2v2M4.93 4.93l1.41 1.41M2 12h2M19 12a5 5 0 0 0-9.33-2.5"/><path d="M17.5 19a4.5 4.5 0 1 0-1.2-8.84A5 5 0 1 0 8 17.5"/>',
    "heart": '<path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/>',
    "tag": '<path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"/><circle cx="7.5" cy="7.5" r="1.5"/>',
    "scroll-text": '<path d="M8 21h12a2 2 0 0 0 2-2v-2H10v2a2 2 0 1 1-4 0V5a2 2 0 1 0-4 0v3h4"/><path d="M15 8h5M15 12h5"/>',
    "gauge": '<path d="m12 14 4-4"/><path d="M3.34 19a10 10 0 1 1 17.32 0"/>',
    "sliders-horizontal": '<path d="M10 5H3M21 5h-7M14 12H3M21 12h-3M16 19H3M21 19h-1"/><circle cx="12" cy="5" r="2"/><circle cx="18" cy="12" r="2"/><circle cx="18" cy="19" r="2"/>',
    "menu": '<path d="M4 6h16M4 12h16M4 18h16"/>',
    "log-out": '<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><path d="m16 17 5-5-5-5M21 12H9"/>',
    "key-round": '<path d="M2.5 9.5a6.5 6.5 0 1 1 11.4 4.3L21 21l-2.5.5-.5-2.5-2 .5-.5-2-2 .5L12 15.8A6.5 6.5 0 0 1 2.5 9.5z"/><circle cx="8.5" cy="9.5" r="1.5"/>',
    "sparkles": '<path d="M12 3v4M12 17v4M3 12h4M17 12h4"/><path d="m5.6 5.6 2.8 2.8M15.6 15.6l2.8 2.8M5.6 18.4l2.8-2.8M15.6 8.4l2.8-2.8"/><circle cx="12" cy="12" r="2"/>',
    "boxes": '<path d="M2.97 12.92A2 2 0 0 0 2 14.63v3.24a2 2 0 0 0 .97 1.71l3 1.8a2 2 0 0 0 2.06 0L12 19v-5.5l-5-3-4.03 2.42Z"/><path d="m7 16.5-4.74-2.85"/><path d="m7 16.5 5 3"/><path d="M7 16.5v5.17"/><path d="M12 13.5V19l3.97 2.38a2 2 0 0 0 2.06 0l3-1.8a2 2 0 0 0 .97-1.71v-3.24a2 2 0 0 0-.97-1.71L17 10.5l-5 3Z"/><path d="m17 16.5-5-3"/><path d="m17 16.5 4.74-2.85"/><path d="M17 16.5v5.17"/><path d="M7.97 4.42A2 2 0 0 0 7 6.13v4.37l5 3 5-3V6.13a2 2 0 0 0-.97-1.71l-3-1.8a2 2 0 0 0-2.06 0l-3 1.8Z"/><path d="M12 8 7.26 5.15"/><path d="m12 8 4.74-2.85"/><path d="M12 13.5V8"/>',
    "radar": '<path d="M19.07 4.93A10 10 0 0 0 6.99 3.34"/><path d="M4 6h.01"/><path d="M2.29 9.62A10 10 0 1 0 21.31 8.35"/><path d="M16.24 7.76A6 6 0 1 0 8.23 16.67"/><path d="M12 12h.01"/>',
    "wrench": '<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>',
}

# Alternate filled-friendly simplified shapes (used for filled + pixel)
FILLED_SHAPES: dict[str, str] = {
    "sun": '<circle cx="12" cy="12" r="5"/><path d="M12 1v2M12 21v2M4.2 4.2l1.4 1.4M18.4 18.4l1.4 1.4M1 12h2M21 12h2M4.2 19.8l1.4-1.4M18.4 5.6l1.4-1.4" fill="none"/>',
    "moon": '<path d="M20 15a7 7 0 1 1-9-9 6.5 6.5 0 0 0 9 9z"/>',
    "cloud": '<path d="M17.5 19H7a5 5 0 0 1-.7-9.95A7 7 0 0 1 19.8 11 4.5 4.5 0 0 1 17.5 19z"/>',
    "cpu": '<rect x="5" y="5" width="14" height="14" rx="2"/><rect x="9" y="9" width="6" height="6" fill="none" stroke="currentColor" stroke-width="1.5"/>',
    "hard-drive": '<path d="M2 12h20l-3.2-6.4A2 2 0 0 0 16.9 4H7.1a2 2 0 0 0-1.8 1.1L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6z"/><circle cx="6" cy="16" r="1.2" fill="none" stroke="currentColor"/>',
    "activity": '<path d="M2 12h3.5l2.5-7 4 14 3-9H22" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>',
    "wifi": '<path d="M12 20a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3z"/><path d="M8.5 15.5a5 5 0 0 1 7 0"/><path d="M5 12a9 9 0 0 1 14 0"/><path d="M2 8.5a13 13 0 0 1 20 0" fill="none" stroke="currentColor" stroke-width="2"/>',
    "settings": '<path d="M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7zm0-6.5 1.2 2.4 2.7.4-2 2 .5 2.7L12 11l-2.4 1.3.5-2.7-2-2 2.7-.4z"/>',
    "layout-grid": '<rect x="3" y="3" width="8" height="8" rx="1"/><rect x="13" y="3" width="8" height="8" rx="1"/><rect x="13" y="13" width="8" height="8" rx="1"/><rect x="3" y="13" width="8" height="8" rx="1"/>',
    "layout-template": '<path d="M3 3h18v6H3zm0 8h6v10H3zm8 0h10v10H11z"/>',
    "search": '<circle cx="11" cy="11" r="7"/><path d="m16.5 16.5 4 4" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/>',
    "plus": '<path d="M11 5h2v6h6v2h-6v6h-2v-6H5v-2h6z"/>',
    "bell": '<path d="M12 2a6 6 0 0 0-6 6v3.5c0 .8-.3 1.5-.8 2.1L4 15h16l-1.2-1.4c-.5-.6-.8-1.3-.8-2.1V8a6 6 0 0 0-6-6zm-2 17a2 2 0 0 0 4 0"/>',
    "users": '<circle cx="9" cy="7" r="3.5"/><path d="M2 20c0-3.3 3.1-5.5 7-5.5s7 2.2 7 5.5"/><circle cx="17" cy="8" r="2.5"/><path d="M16 20c0-1.8 1.2-3.3 3-4.2"/>',
    "rss": '<circle cx="5" cy="19" r="2"/><path d="M4 11a9 9 0 0 1 9 9" fill="none" stroke="currentColor" stroke-width="2.5"/><path d="M4 4a16 16 0 0 1 16 16" fill="none" stroke="currentColor" stroke-width="2.5"/>',
    "server": '<rect x="3" y="3" width="18" height="7" rx="1.5"/><rect x="3" y="14" width="18" height="7" rx="1.5"/><circle cx="7" cy="6.5" r="1" fill="none" stroke="currentColor"/><circle cx="7" cy="17.5" r="1" fill="none" stroke="currentColor"/>',
    "plug": '<path d="M9 2v5h6V2H9zm-2 7h10v3a5 5 0 0 1-4 4.9V22h-2v-5.1A5 5 0 0 1 7 12V9z"/>',
    "home": '<path d="M12 3 2 11h2v10h7v-7h2v7h7V11h2z"/>',
    "shield": '<path d="M12 2 4 5v7c0 5.2 3.4 9.1 8 10.5 4.6-1.4 8-5.3 8-10.5V5z"/>',
    "shield-check": '<path d="M12 2 4 5v7c0 5.2 3.4 9.1 8 10.5 4.6-1.4 8-5.3 8-10.5V5z"/><path d="m9 12 2 2 4-4" fill="none" stroke="#0b1220" stroke-width="2" stroke-linecap="round"/>',
    "database": '<ellipse cx="12" cy="5" rx="8" ry="3"/><path d="M4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5"/><path d="M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3" fill="none" stroke="currentColor" stroke-width="1.5"/>',
    "zap": '<path d="M13 2 4 14h7l-1 8 10-12h-7l0-8z"/>',
    "eye": '<path d="M12 5C6 5 2 12 2 12s4 7 10 7 10-7 10-7-4-7-10-7zm0 10a3 3 0 1 1 0-6 3 3 0 0 1 0 6z"/>',
    "palette": '<path d="M12 2a10 10 0 0 0 0 20h1.5a1.5 1.5 0 0 0 0-3H16a4 4 0 0 0 0-8 10 10 0 0 0-4-9z"/><circle cx="7.5" cy="11" r="1.2" fill="#0b1220"/><circle cx="10" cy="7.5" r="1.2" fill="#0b1220"/><circle cx="14.5" cy="7" r="1.2" fill="#0b1220"/>',
    "arrow-left": '<path d="M11 5 4 12l7 7v-5h9v-4h-9z"/>',
    "external-link": '<path d="M14 3h7v7h-2V6.4l-9.3 9.3-1.4-1.4L17.6 5H14z"/><path d="M5 5h6v2H7v10h10v-4h2v6H5z"/>',
    "power": '<path d="M11 2h2v10h-2z"/><path d="M7.05 6.05a7 7 0 1 0 9.9 0l-1.4 1.4a5 5 0 1 1-7.1 0z"/>',
    "play": '<path d="M7 4v16l13-8z"/>',
    "pause": '<rect x="5" y="4" width="5" height="16" rx="1"/><rect x="14" y="4" width="5" height="16" rx="1"/>',
    "refresh": '<path d="M17.7 6.3A8 8 0 1 0 20 12h-2a6 6 0 1 1-1.7-4.2L14 10h6V4z"/>',
    "cloud-sun": '<circle cx="9" cy="8" r="3"/><path d="M17.5 19H8a4.5 4.5 0 1 1 .9-8.9A5.5 5.5 0 0 1 19.2 12 3.5 3.5 0 0 1 17.5 19z"/>',
    "heart": '<path d="M12 21s-7-4.5-9.5-9A5.5 5.5 0 0 1 12 6a5.5 5.5 0 0 1 9.5 6C19 16.5 12 21 12 21z"/>',
    "tag": '<path d="M11.2 2H4a2 2 0 0 0-2 2v7.2a2 2 0 0 0 .6 1.4l8.7 8.7a2.4 2.4 0 0 0 3.4 0l6.6-6.6a2.4 2.4 0 0 0 0-3.4z"/><circle cx="7.5" cy="7.5" r="1.5" fill="#0b1220"/>',
    "scroll-text": '<path d="M8 3h9a2 2 0 0 1 2 2v12H10V5a2 2 0 0 0-2-2zm-4 4h2v12a2 2 0 0 0 2 2h10v2H8a4 4 0 0 1-4-4z"/><path d="M12 8h5M12 12h5" fill="none" stroke="#0b1220" stroke-width="1.5"/>',
    "gauge": '<path d="M12 13a2 2 0 0 0 1.7-1L17 7a9 9 0 1 0-1.7 12.5"/><path d="m12 13 4-5" fill="none" stroke="#0b1220" stroke-width="2"/>',
    "sliders-horizontal": '<path d="M3 5h7v2H3zm10 0h8v2h-8zM3 11h13v2H3zm15 0h3v2h-3zM3 17h11v2H3zm13 0h5v2h-5z"/><circle cx="12" cy="6" r="2.2"/><circle cx="18" cy="12" r="2.2"/><circle cx="16" cy="18" r="2.2"/>',
    "menu": '<path d="M3 5h18v2.5H3zm0 5.75h18v2.5H3zm0 5.75h18V19H3z"/>',
    "log-out": '<path d="M3 4h8v16H3zm10 6h5.2l-1.6-1.6L18 7l4 4-4 4-1.4-1.4L18.2 12H13z"/>',
    "key-round": '<circle cx="8.5" cy="9.5" r="4.5"/><circle cx="8.5" cy="9.5" r="1.5" fill="#0b1220"/><path d="m12 12 8 8v2h-2l-.5-2-2 .5-.5-2-2 .5z"/>',
    "sparkles": '<path d="M12 2 13.5 8 19 9.5 13.5 11 12 17l-1.5-6L5 9.5 10.5 8z"/><path d="m18 14 .8 2.2L21 17l-2.2.8L18 20l-.8-2.2L15 17l2.2-.8z"/>',
    "boxes": '<path d="M12 3 4 7v4l8 4 8-4V7zm0 10-8-4v8l8 4 8-4v-8z"/>',
    "radar": '<circle cx="12" cy="12" r="2"/><circle cx="12" cy="12" r="6" fill="none" stroke="currentColor" stroke-width="2"/><circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2"/><path d="M12 12 18 6" fill="none" stroke="currentColor" stroke-width="2"/>',
    "wrench": '<path d="M14.7 6.3a4 4 0 0 1 5 5l-7.5 7.5a2 2 0 0 1-2.8 0l-1.4-1.4 7.5-7.5a4 4 0 0 1-5-5l2.5 2.5 1.7-1.6z"/>',
}


def outline_svg(inner: str, *, width: str = "2", cap: str = "round", join: str = "round") -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" '
        f'stroke="currentColor" stroke-width="{width}" stroke-linecap="{cap}" '
        f'stroke-linejoin="{join}">{inner}</svg>'
    )


def filled_svg(inner: str) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" '
        f'fill="currentColor">{inner}</svg>'
    )


def duotone_svg(inner: str) -> str:
    # Stroke primary + soft fill behind common shapes
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" '
        f'stroke="currentColor" stroke-width="1.75" stroke-linecap="round" '
        f'stroke-linejoin="round">'
        f'<g opacity="0.28" fill="currentColor" stroke="none">{inner}</g>'
        f'<g fill="none" stroke="currentColor">{inner}</g>'
        f"</svg>"
    )


def pixel_svg(inner: str) -> str:
    # Chunky square geometry via thicker square strokes on simplified paths
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" '
        f'stroke="currentColor" stroke-width="2.75" stroke-linecap="square" '
        f'stroke-linejoin="miter" shape-rendering="crispEdges">{inner}</svg>'
    )


STYLE_BUILDERS = {
    "outline-round": lambda name: outline_svg(PATHS[name], width="1.75", cap="round", join="round"),
    "bold-square": lambda name: outline_svg(PATHS[name], width="2.75", cap="square", join="miter"),
    "filled": lambda name: filled_svg(FILLED_SHAPES.get(name, PATHS[name])),
    "duotone": lambda name: duotone_svg(PATHS[name]),
    "pixel-geo": lambda name: pixel_svg(PATHS[name]),
}

ICON_KEYS = sorted(PATHS.keys())

THEME_STYLE: dict[str, str] = {
    "default": "outline-round",
    "crimson-flare": "bold-square",
    "dracula": "duotone",
    "nord": "outline-round",
    "cyberpunk-neon": "duotone",
    "catppuccin-mocha": "filled",
    "gruvbox-dark": "bold-square",
    "tokyo-night": "duotone",
    "one-dark": "outline-round",
    "everforest": "outline-round",
    "monokai": "filled",
    "rose-pine": "filled",
    "solarized-dark": "outline-round",
    "terminal-phosphor": "pixel-geo",
    "blueprint-tech": "bold-square",
    "luxury-gold": "filled",
    "holographic-prism": "duotone",
    "brutalist-mono": "bold-square",
    "aurora-borealis": "duotone",
    "desert-dusk": "filled",
    "rainforest-mist": "outline-round",
    "volcanic-ember": "bold-square",
    "terminal-matrix": "pixel-geo",
    "sakura-dream": "filled",
    "lavender-mist": "filled",
    "rose-gold-blush": "filled",
    "cotton-candy": "filled",
    "peach-blossom": "filled",
    "nebula-void": "duotone",
    "steampunk-brass": "bold-square",
    "zen-garden": "outline-round",
    "retro-arcade": "pixel-geo",
    "midnight-city": "duotone",
    "taghawsa": "duotone",
    "ember-hearth": "bold-square",
    "neon-boulevard": "duotone",
    "kelp-abyss": "outline-round",
    "amber-console": "pixel-geo",
    "glacier-mist": "outline-round",
    "glow-glass": "duotone",
    "neumorphism": "filled",
}

ALIASES_BY_STYLE: dict[str, dict[str, str]] = {
    "outline-round": {},
    "bold-square": {
        "palette": "moon",
        "gauge": "activity",
        "server": "boxes",
        "eye": "shield",
        "layout-template": "layout-grid",
        "cpu": "wrench",
    },
    "filled": {
        "palette": "sparkles",
        "gauge": "radar",
        "server": "database",
        "eye": "shield-check",
        "layout-template": "home",
        "zap": "power",
    },
    "duotone": {
        "palette": "moon",
        "gauge": "radar",
        "server": "boxes",
        "performance": "activity",
        "eye": "radar",
        "layout-template": "sparkles",
        "cpu": "activity",
        "bell": "zap",
    },
    "pixel-geo": {
        "palette": "sun",
        "gauge": "activity",
        "server": "cpu",
        "eye": "search",
        "layout-template": "layout-grid",
        "shield-check": "shield",
        "database": "hard-drive",
        "zap": "power",
    },
}


def write_styles() -> None:
    for style, builder in STYLE_BUILDERS.items():
        out = STYLES / style
        out.mkdir(parents=True, exist_ok=True)
        for name in ICON_KEYS:
            (out / f"{name}.svg").write_text(builder(name) + "\n", encoding="utf-8")
        print(f"wrote {len(ICON_KEYS)} icons -> {out}")


def rewrite_packs() -> None:
    icons_map = {k: f"{k}.svg" for k in ICON_KEYS}
    for theme, style in THEME_STYLE.items():
        pack_dir = ICONS / theme
        pack_dir.mkdir(parents=True, exist_ok=True)
        pack = {
            "version": 2,
            "base": f"/static/themes/icons/_styles/{style}",
            "icons": icons_map,
            "aliases": ALIASES_BY_STYLE.get(style, {}),
        }
        (pack_dir / "pack.json").write_text(
            json.dumps(pack, indent=2) + "\n", encoding="utf-8"
        )
        print(f"pack {theme} -> {style}")


def main() -> None:
    write_styles()
    rewrite_packs()
    print(f"done: {len(ICON_KEYS)} keys, {len(THEME_STYLE)} themes")


if __name__ == "__main__":
    main()
