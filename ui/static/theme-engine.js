/**
 * AMUD Theme Engine — local icon packs, fonts, wallpaper (manifest v5).
 */
(function (global) {
    'use strict';

    var CHROME_SELECTOR = [
        '.topbar [data-lucide]',
        '.telemetry-bar-container [data-lucide]',
        '.greeting-widget [data-lucide]',
        '.clock-widget [data-lucide]',
        '.weather-widget [data-lucide]',
        '.category-tabs [data-lucide]',
        '.ws-status-pill [data-lucide]',
        '.settings-sidebar [data-lucide]',
        '.settings-sidebar .amud-theme-icon[data-icon-name]',
        '.settings-page-title [data-lucide]',
        '.nav-item [data-lucide]',
        '.nav-item .amud-theme-icon[data-icon-name]',
        '.save-bar [data-lucide]',
        '[data-theme-icon]'
    ].join(', ');

    var manifestCache = null;
    var activeBackgroundScript = null;
    var threeLoadPromise = null;

    function themeId() {
        return document.documentElement?.getAttribute('data-theme-id')
            || document.body?.getAttribute('data-theme-id')
            || 'default';
    }

    function setThemeId(id) {
        var next = id || 'default';
        document.documentElement.setAttribute('data-theme-id', next);
        if (document.body) {
            document.body.setAttribute('data-theme-id', next);
        }
    }

    function resolveAssetUrl(manifest, path) {
        if (!path) return '';
        if (path.indexOf('http://') === 0 || path.indexOf('https://') === 0) return path;
        if (path.indexOf('/') === 0) return path;
        var base = manifest?.assetBase || '';
        if (!base) return '/' + path.replace(/^\//, '');
        return base.replace(/\/$/, '') + '/' + path.replace(/^\//, '');
    }

    function injectLayoutCss(layoutUrl) {
        if (!layoutUrl) return;
        var id = 'amud-theme-layout';
        var existing = document.getElementById(id);
        if (existing && existing.getAttribute('data-href') === layoutUrl) return;
        if (existing) existing.remove();
        var link = document.createElement('link');
        link.id = id;
        link.rel = 'stylesheet';
        link.href = layoutUrl;
        link.setAttribute('data-href', layoutUrl);
        document.head.appendChild(link);
    }

    function applyUiProfile(entry) {
        if (!entry || !entry.uiProfile) return;
        document.body.setAttribute('data-ui-profile', entry.uiProfile);
    }

    function loadManifest() {
        if (manifestCache) return Promise.resolve(manifestCache);
        return fetch('/static/themes/manifest.json', { cache: 'no-store' })
            .then(function (res) {
                if (!res.ok) throw new Error('manifest');
                return res.json();
            })
            .then(function (data) {
                manifestCache = data;
                return data;
            });
    }

    function themeEntry(manifest, id) {
        return (manifest.themes || []).find(function (t) { return t.id === id; });
    }

    function cacheKey(suffix, id) {
        return 'amud-theme-' + (id || themeId()) + '-' + suffix;
    }

    function injectFont(fontUrl) {
        if (!fontUrl) return;
        var id = 'amud-theme-font';
        var existing = document.getElementById(id);
        if (existing && existing.getAttribute('data-href') === fontUrl) return;
        if (existing) existing.remove();
        var link = document.createElement('link');
        link.id = id;
        link.rel = 'stylesheet';
        link.href = fontUrl;
        link.setAttribute('data-href', fontUrl);
        document.head.appendChild(link);
    }

    function applyWallpaperUrl(url) {
        if (!url) return;
        document.documentElement.style.setProperty('--brand-bg-image', "url('" + url.replace(/'/g, "\\'") + "')");
    }

    function fetchSvg(url, id) {
        var key = cacheKey('svg:' + url, id);
        try {
            var cached = sessionStorage.getItem(key);
            if (cached) return Promise.resolve(cached);
        } catch (e) { /* ignore */ }
        return fetch(url, { cache: 'force-cache' })
            .then(function (res) {
                if (!res.ok) throw new Error('svg');
                return res.text();
            })
            .then(function (text) {
                try { sessionStorage.setItem(key, text); } catch (e) { /* ignore */ }
                return text;
            });
    }

    function replaceIconNode(el, svgText, name) {
        var wrap = document.createElement('span');
        wrap.className = 'amud-theme-icon';
        wrap.setAttribute('data-icon-name', name);
        wrap.innerHTML = svgText;
        var svg = wrap.querySelector('svg');
        if (svg) {
            var size = el.getAttribute('data-icon-size') || el.style.width || '1.25rem';
            if (typeof size === 'string' && size) {
                svg.style.width = size;
                svg.style.height = size;
            }
            svg.setAttribute('aria-hidden', 'true');
        }
        el.replaceWith(wrap);
    }

    function iconNameFromEl(el) {
        return el.getAttribute('data-lucide')
            || el.getAttribute('data-theme-icon')
            || el.getAttribute('data-icon-name');
    }

    function loadIconPack(manifest, id) {
        var entry = themeEntry(manifest, id);
        if (!entry || !entry.iconPack) return Promise.resolve(null);
        var packUrl = resolveAssetUrl(manifest, entry.iconPack);
        var key = cacheKey('pack:v2:' + packUrl, id);
        try {
            var cached = sessionStorage.getItem(key);
            if (cached) return Promise.resolve(JSON.parse(cached));
        } catch (e) { /* ignore */ }
        return fetch(packUrl, { cache: 'no-cache' })
            .then(function (res) {
                if (!res.ok) throw new Error('pack');
                return res.json();
            })
            .then(function (pack) {
                try { sessionStorage.setItem(key, JSON.stringify(pack)); } catch (e) { /* ignore */ }
                return pack;
            });
    }

    function resolvePackIcon(pack, logicalName) {
        if (!logicalName || !pack || !pack.icons) return null;
        var aliases = pack.aliases || {};
        var resolved = aliases[logicalName] || logicalName;
        if (!pack.icons[resolved]) return null;
        return { logical: logicalName, resolved: resolved, file: pack.icons[resolved] };
    }

    function swapChromeIcons(manifest, pack) {
        if (!pack || !pack.icons) return Promise.resolve();
        var base = resolveAssetUrl(manifest, pack.base || '');
        var nodes = document.querySelectorAll(CHROME_SELECTOR);
        var tasks = [];
        nodes.forEach(function (el) {
            if (el.closest('.app-card')) return;
            var name = iconNameFromEl(el);
            var mapped = resolvePackIcon(pack, name);
            if (!mapped) return;
            var file = mapped.file;
            var url = file.indexOf('http') === 0 ? file : base.replace(/\/$/, '') + '/' + file.replace(/^\//, '');
            tasks.push(
                fetchSvg(url, themeId() + ':' + mapped.resolved).then(function (svg) {
                    replaceIconNode(el, svg, mapped.logical);
                }).catch(function () { /* keep lucide */ })
            );
        });
        return Promise.all(tasks);
    }

    function teardownBackground() {
        if (global.amudThemeBackground && typeof global.amudThemeBackground.destroy === 'function') {
            try { global.amudThemeBackground.destroy(); } catch (e) { /* ignore */ }
        }
        activeBackgroundScript = null;
    }

    function loadScriptOnce(src, globalFlag) {
        if (globalFlag && global[globalFlag]) return Promise.resolve();
        var existing = document.querySelector('script[data-amud-src="' + src + '"]');
        if (existing) {
            if (existing.getAttribute('data-amud-loaded') === '1') {
                return Promise.resolve();
            }
            return new Promise(function (resolve, reject) {
                existing.addEventListener('load', function () {
                    existing.setAttribute('data-amud-loaded', '1');
                    resolve();
                }, { once: true });
                existing.addEventListener('error', reject, { once: true });
            });
        }
        return new Promise(function (resolve, reject) {
            var script = document.createElement('script');
            script.src = src;
            script.setAttribute('data-amud-src', src);
            script.onload = function () {
                script.setAttribute('data-amud-loaded', '1');
                if (globalFlag) global[globalFlag] = true;
                resolve();
            };
            script.onerror = reject;
            document.head.appendChild(script);
        });
    }

    function loadThreeJs() {
        if (typeof global.THREE !== 'undefined') return Promise.resolve();
        if (threeLoadPromise) return threeLoadPromise;
        threeLoadPromise = loadScriptOnce('/static/vendor/three.min.js', '__amudThreeLoaded');
        return threeLoadPromise;
    }

    function loadBackgroundScript(manifest, entry) {
        teardownBackground();
        if (!entry || !entry.backgroundScript) return Promise.resolve();
        var scriptUrl = resolveAssetUrl(manifest, entry.backgroundScript);
        if (!scriptUrl) return Promise.resolve();

        return loadThreeJs()
            .then(function () {
                return loadScriptOnce(scriptUrl).then(function () {
                    activeBackgroundScript = scriptUrl;
                    if (global.amudThemeBackground && typeof global.amudThemeBackground.init === 'function') {
                        global.amudThemeBackground.init();
                    }
                });
            })
            .catch(function () { /* WebGL optional */ });
    }

    function applyThemeChrome(id) {
        return loadManifest().then(function (manifest) {
            var entry = themeEntry(manifest, id);
            applyUiProfile(entry);
            if (entry?.layoutCss) injectLayoutCss(entry.layoutCss);
            if (entry?.fontUrl) injectFont(entry.fontUrl);
            if (entry?.wallpaper && entry.usesWallpaper !== false) {
                var wp = resolveAssetUrl(manifest, entry.wallpaper);
                if (wp) applyWallpaperUrl(wp);
            } else if (entry && entry.usesWallpaper === false) {
                document.documentElement.style.removeProperty('--brand-bg-image');
            }
            return loadIconPack(manifest, id).then(function (pack) {
                return swapChromeIcons(manifest, pack);
            }).then(function () {
                return loadBackgroundScript(manifest, entry);
            });
        });
    }

    function refreshBackground(overrideId) {
        var id = overrideId || themeId();
        return loadManifest().then(function (manifest) {
            var entry = themeEntry(manifest, id);
            return loadBackgroundScript(manifest, entry);
        }).catch(function () { /* ignore */ });
    }

    function initThemeEngine() {
        var id = themeId();
        document.body.classList.add('theme-' + id);
        return applyThemeChrome(id).catch(function () { /* offline / default */ });
    }

    function refreshChromeIcons(overrideId) {
        var id = overrideId || themeId();
        if (overrideId) {
            setThemeId(id);
        }
        return applyThemeChrome(id).catch(function () { /* ignore */ });
    }

    global.amudThemeEngine = {
        themeId: themeId,
        setThemeId: setThemeId,
        resolveAssetUrl: resolveAssetUrl,
        loadManifest: loadManifest,
        applyUiProfile: applyUiProfile,
        injectLayoutCss: injectLayoutCss,
        refreshChromeIcons: refreshChromeIcons,
        refreshBackground: refreshBackground,
        init: initThemeEngine
    };

    var resizeBgTimer = null;
    function scheduleBackgroundRefresh() {
        if (resizeBgTimer) global.clearTimeout(resizeBgTimer);
        resizeBgTimer = global.setTimeout(function () {
            if (global.amudThemeBackground && typeof global.amudThemeBackground.refresh === 'function') {
                try { global.amudThemeBackground.refresh(); } catch (e) { /* ignore */ }
                return;
            }
            refreshBackground().catch(function () { /* ignore */ });
        }, 250);
    }

    global.addEventListener('resize', scheduleBackgroundRefresh);
    global.addEventListener('orientationchange', scheduleBackgroundRefresh);

    global.addEventListener('load', function () {
        if (themeId() !== 'taghawsa') return;
        if (document.body.classList.contains('has-webgl-bg')) return;
        refreshBackground().catch(function () { /* ignore */ });
    });

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', function () {
            initThemeEngine().then(function () {
                if (typeof lucide !== 'undefined' && lucide.createIcons) {
                    lucide.createIcons();
                }
            });
        });
    } else {
        initThemeEngine().then(function () {
            if (typeof lucide !== 'undefined' && lucide.createIcons) {
                lucide.createIcons();
            }
        });
    }
})(typeof window !== 'undefined' ? window : globalThis);
