---
sidebar_position: 6
---

# Custom Themes

AMUD supports full visual customization through **Custom CSS injection**. Copy CSS from the gallery and paste it into **Settings → Appearance → Custom CSS**.

:::tip Interactive Gallery
Browse, search, and **preview** all themes on the **[Theme Gallery](/themes)** page. Each card shows a screenshot so you can see how the theme looks before applying it. Click **Copy CSS** or **Download CSS**, then paste into your dashboard settings.
:::

:::tip Bundled theme assets (manifest v5)
Theme **CSS**, **icons**, **wallpapers**, and **gallery previews** all ship in `ui.tar.gz` under `/static/themes/` — **fully offline** after install. No jsDelivr or GitHub CDN required.
:::

:::tip Offline default
**AMUD Default** uses the local `/static/wallpaper.png` and Lucide icons. Other themes use bundled WebP wallpapers, per-profile icon packs, and layout CSS from `/static/theme-layouts/`.
:::

:::tip Combining with Built-in Settings
Themes work alongside built-in quick settings (accent color, wallpaper URL, glass blur, grid columns, etc.). If a theme sets `--accent-color` but you also pick a color in the UI, the UI setting wins because it is injected into `:root` after custom CSS. Remove `--accent-color` from the theme CSS to control accent from the UI, or leave Custom CSS empty and use quick controls only. Wallpaper tint and background styling come from theme CSS or your wallpaper URL.
:::

:::info Recovery
If a theme breaks your layout, see [Recovering from Broken Custom CSS](./troubleshooting.md#recovering-from-broken-custom-css).
:::

---

## Available Themes

| Theme | Style |
|-------|-------|
| AMUD Default | Built-in orange glass — leave Custom CSS empty |
| Dracula | Dark purple hacker |
| Nord | Arctic blue, calm |
| Cyberpunk Neon | Neon pink on black + scanlines |
| Catppuccin Mocha | Soft pastel lavender |
| Gruvbox Dark | Warm retro terminal |
| Azure Calm | Deep blue, calm (`tokyo-night` id) |
| One Dark | Classic Atom palette |
| Everforest | Muted green forest |
| Monokai | Neon green developer |
| Rose Pine | Elegant rose & pine |
| Solarized Dark | Low-contrast scientific |
| Ember Hearth | Warm copper on deep charcoal |
| Neon Boulevard | Magenta/cyan nightlife |
| Kelp Abyss | Teal lagoon depth |
| Glacier Mist | Cool blue-gray frost |
| **Terminal Phosphor** ★ | CRT green monospace + scanlines |
| **Amber Console** ★ | Soft amber CRT on charcoal |
| **Terminal Matrix** ★ | Matrix green drift |
| **Blueprint Tech** ★ | Engineering schematic cyan |
| **Luxury Gold** ★ | Obsidian + gold serif headers |
| **Holographic Prism** ★ | Animated iridescent borders |
| **Glow and Glass** ★ | Cyan glass + glow (CSS shell / layout profile) |
| **Neumorphism** ★ | Soft extruded clay UI (CSS shell / layout profile) |
| **Brutalist Mono** ★ | Raw concrete, no blur, bold mono |
| **Taghawsa** ★ | WebGL Moroccan orange/teal gradient |
| **Nature pack** | Aurora, desert, rainforest, volcanic |
| **Feminine pack** | Sakura, lavender, rose gold, cotton candy, peach |
| **Variety pack** | Nebula, steampunk, zen, arcade, midnight city |

★ = distinct UI profiles (layout, typography, icons, or WebGL) — not just color swaps

**Bundled assets:** `ui/static/themes/` (CSS, WebP wallpapers/previews, SVG icon packs) and `ui/static/theme-layouts/` ship in every release `ui.tar.gz`.

**Raw CSS files:** [docs/static/themes/](https://github.com/boubli/AMUD-Dashboard/tree/main/docs/static/themes) (docs site) and **`ui/static/themes/`** (shipped with your dashboard for offline use).

Open the [Theme Gallery](/themes) for search, previews, **Copy CSS**, **Download CSS**, and **Copy wallpaper**.

---

## Creating Your Own Theme

Override CSS variables in **Settings → Custom CSS**:

```css
/* My Custom AMUD Theme */
:root {
    --accent-color: #your-color;
    --accent-glow: rgba(r, g, b, 0.15);
    --bg-card: rgba(r, g, b, 0.70);
    --border-card: rgba(r, g, b, 0.10);
    --border-hover: rgba(r, g, b, 0.25);
    --text-primary: #your-text;
    --text-secondary: #your-secondary;
    --text-muted: #your-muted;
    --success: #your-green;
    --danger: #your-red;
}

body {
    background-color: #your-base-bg;
}
```

### Available CSS Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `--accent-color` | Primary brand/accent color | `#cf6427` |
| `--accent-glow` | Subtle glow tint derived from accent | `rgba(207,100,39,0.15)` |
| `--bg-card` | Glass card background | `rgba(15,20,25,0.45)` |
| `--border-card` | Card border at rest | `rgba(255,255,255,0.08)` |
| `--border-hover` | Card border on hover | `rgba(255,255,255,0.16)` |
| `--text-primary` | Main text color | `#f8fafc` |
| `--text-secondary` | Secondary/label text | `#94a3b8` |
| `--text-muted` | Subtle/hint text | `#64748b` |
| `--success` | Online/healthy status | `#10b981` |
| `--danger` | Error/stopped status | `#ef4444` |
| `--glass-blur-intensity` | Backdrop blur amount | `16px` |
| `--glass-opacity` | Glass panel opacity | `0.45` |
| `--radius-xl` | Large border radius | `16px` |

Advanced themes can also style `.glass-panel`, `.header-title`, `body::before`, and animations — see `terminal-phosphor.css` or `neon-boulevard.css` for examples.

### Adding a theme to the gallery

1. Add `your-theme.css` to `docs/static/themes/` **and** `ui/static/themes/`
2. Add a preview PNG to `docs/static/themes/assets/AMUD-Theme-Your-Theme.png`
3. Add `docs/src/data/themes/definitions/your-theme.ts` and register it in `definitions/index.ts`
4. Add an entry to `ui/static/themes/manifest.json`

After merging to `main`, the theme appears on GitHub Pages and ships offline with the next AMUD release.

### Catppuccin Frappé

Choose **Catppuccin Frappé** in **Settings → Appearance → Theme Gallery** for
muted blue-gray surfaces, pastel text, and mauve accents from the
[official Catppuccin palette](https://catppuccin.com/palette/). The theme uses
an offline CSS gradient rather than a wallpaper download. Its light-mode
variant uses the matching Catppuccin Latte palette.
