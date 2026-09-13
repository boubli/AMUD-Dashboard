(function () {
    const MODE_KEY = 'amud_search_mode';
    const ENGINE_KEY = 'amud_search_engine';

    function readStorage(key, fallback) {
        try {
            return localStorage.getItem(key) || fallback;
        } catch (err) {
            console.warn('localStorage read failed:', err);
            return fallback;
        }
    }

    function writeStorage(key, value) {
        try {
            localStorage.setItem(key, value);
        } catch (err) {
            console.warn('localStorage write failed:', err);
        }
    }

    function getMode() {
        return readStorage(MODE_KEY, 'apps');
    }

    function setMode(mode) {
        writeStorage(MODE_KEY, mode);
        refreshModeUi();
    }

    function readSearchConfig() {
        const el = document.getElementById('amud-search-config');
        if (!el) {
            return {
                default: 'google',
                engines: [
                    { id: 'google', name: 'Google', url: 'https://www.google.com/search?q={query}' },
                    { id: 'bing', name: 'Bing', url: 'https://www.bing.com/search?q={query}' },
                    { id: 'duckduckgo', name: 'DuckDuckGo', url: 'https://duckduckgo.com/?q={query}' },
                    { id: 'youtube', name: 'YouTube', url: 'https://www.youtube.com/results?search_query={query}' },
                    { id: 'github', name: 'GitHub', url: 'https://github.com/search?q={query}' },
                ],
            };
        }
        try {
            const cfg = JSON.parse(el.textContent || '{}');
            return {
                default: cfg.default || 'google',
                engines: Array.isArray(cfg.engines) ? cfg.engines : [],
            };
        } catch (err) {
            console.warn('amud-search-config parse failed:', err);
            return { default: 'google', engines: [] };
        }
    }

    function buildSearchUrl(template, query) {
        const q = encodeURIComponent(query);
        if (!template) return 'https://www.google.com/search?q=' + q;
        return String(template).split('{query}').join(q).split('%s').join(q);
    }

    function engineUrlById(id) {
        const cfg = readSearchConfig();
        const found = cfg.engines.find(function (e) { return e.id === id; });
        return found ? found.url : null;
    }

    function updateEngineVisibility(mode, engineSelect, divider) {
        const show = mode === 'web';
        if (engineSelect) {
            engineSelect.style.display = show ? '' : 'none';
        }
        if (divider) {
            divider.style.display = show ? '' : 'none';
        }
    }

    function updateSearchInputForMode(mode, input) {
        if (!input) return;
        input.placeholder = mode === 'web' ? 'Search the web...' : 'Search apps... (Ctrl+K)';
        input.ariaLabel = mode === 'web' ? 'Search the web' : 'Search apps';
    }

    function updateModeButtons(mode, modeApps, modeWeb) {
        if (modeApps) {
            modeApps.classList.toggle('active', mode === 'apps');
            modeApps.setAttribute('aria-pressed', mode === 'apps' ? 'true' : 'false');
        }
        if (modeWeb) {
            modeWeb.classList.toggle('active', mode === 'web');
            modeWeb.setAttribute('aria-pressed', mode === 'web' ? 'true' : 'false');
        }
    }

    function isMobileSearchCollapsed() {
        const wrapper = document.getElementById('search-bar-wrapper');
        if (!wrapper) return false;
        return globalThis.matchMedia('(max-width: 768px)').matches && !wrapper.classList.contains('is-expanded');
    }

    function expandSearchBar() {
        const wrapper = document.getElementById('search-bar-wrapper');
        const toggle = document.getElementById('search-toggle');
        if (!wrapper) return;
        wrapper.classList.add('is-expanded');
        if (toggle) toggle.setAttribute('aria-expanded', 'true');
    }

    function collapseSearchBar() {
        const wrapper = document.getElementById('search-bar-wrapper');
        const toggle = document.getElementById('search-toggle');
        if (!wrapper) return;
        wrapper.classList.remove('is-expanded');
        if (toggle) toggle.setAttribute('aria-expanded', 'false');
    }

    function initSearchToggle() {
        const wrapper = document.getElementById('search-bar-wrapper');
        const toggle = document.getElementById('search-toggle');
        if (!wrapper || !toggle) return;

        toggle.addEventListener('click', function () {
            expandSearchBar();
            const input = document.getElementById('search-input');
            if (input) {
                input.focus();
            }
        });
    }

    function applyPreferredEngine() {
        const select = document.getElementById('search-engine');
        if (!select) return;
        const cfg = readSearchConfig();
        const stored = readStorage(ENGINE_KEY, '');
        const preferred = stored || cfg.default || 'google';
        const hasOption = Array.from(select.options).some(function (o) { return o.value === preferred; });
        select.value = hasOption ? preferred : (cfg.default || 'google');
    }

    function refreshModeUi() {
        const mode = getMode();
        const engineSelect = document.getElementById('search-engine');
        const divider = document.querySelector('.search-input-divider');
        const input = document.getElementById('search-input');
        const modeApps = document.getElementById('search-mode-apps');
        const modeWeb = document.getElementById('search-mode-web');
        updateEngineVisibility(mode, engineSelect, divider);
        updateSearchInputForMode(mode, input);
        updateModeButtons(mode, modeApps, modeWeb);
    }

    function applyCategoryVisibility(card, category) {
        const show = category === 'all' || card.dataset.category === category;
        card.style.display = show ? 'flex' : 'none';
        return show;
    }

    function getCardSearchText(card) {
        const name = (card.dataset.appName || '').toLowerCase();
        const title = (card.querySelector('.app-card-title, .feed-card-title')?.textContent || '').toLowerCase();
        return name + '\n' + title;
    }

    function getOrCreateEmptyMessage(grid) {
        let emptyMsg = grid.querySelector('.filter-empty-msg');
        if (!emptyMsg) {
            emptyMsg = document.createElement('div');
            emptyMsg.className = 'glass-panel filter-empty-msg ' + (grid.classList.contains('feeds-grid') ? 'feed-card' : 'app-card');
            grid.appendChild(emptyMsg);
        }
        return emptyMsg;
    }

    function applyAppFilter(query) {
        const q = (query || '').trim().toLowerCase();
        const cardSelector = '.app-card:not(.filter-empty-msg), .feed-card:not(.filter-empty-msg)';
        const cards = document.querySelectorAll(cardSelector);
        const category = globalThis.activeCategoryFilter || 'all';
        let visible = 0;

        cards.forEach(function (card) {
            if (!q) {
                if (applyCategoryVisibility(card, category)) visible += 1;
                return;
            }
            const match = getCardSearchText(card).includes(q);
            card.style.display = match ? 'flex' : 'none';
            if (match) visible += 1;
        });

        const grid = document.querySelector('main.bento-grid, .feeds-grid');
        if (!grid) return;
        if (q && visible === 0) {
            const emptyMsg = getOrCreateEmptyMessage(grid);
            emptyMsg.textContent = 'No apps match "' + query.trim() + '"';
            emptyMsg.style.display = 'flex';
        } else if (q) {
            const emptyMsg = grid.querySelector('.filter-empty-msg');
            if (!emptyMsg) return;
            emptyMsg.style.display = 'none';
        }
    }

    function openWebSearch(query) {
        const engine = document.getElementById('search-engine')?.value || 'google';
        const url = buildSearchUrl(engineUrlById(engine) || 'https://www.google.com/search?q={query}', query);
        writeStorage(ENGINE_KEY, engine);
        globalThis.open(url, '_blank');
    }

    function init() {
        const input = document.getElementById('search-input');
        const modeApps = document.getElementById('search-mode-apps');
        const modeWeb = document.getElementById('search-mode-web');
        const engineSelect = document.getElementById('search-engine');
        if (!input) return;

        applyPreferredEngine();
        refreshModeUi();
        if (modeApps) modeApps.addEventListener('click', function () { setMode('apps'); });
        if (modeWeb) modeWeb.addEventListener('click', function () { setMode('web'); });
        if (engineSelect) {
            engineSelect.addEventListener('change', function () {
                writeStorage(ENGINE_KEY, engineSelect.value);
            });
        }

        input.addEventListener('input', function () {
            if (getMode() === 'apps') {
                applyAppFilter(input.value);
            }
        });

        input.addEventListener('keydown', function (e) {
            if (e.key === 'Escape') {
                input.value = '';
                applyAppFilter('');
                input.blur();
                if (globalThis.matchMedia('(max-width: 768px)').matches) {
                    collapseSearchBar();
                }
                return;
            }
            if (e.key !== 'Enter') return;
            const query = input.value.trim();
            if (!query) return;
            if (getMode() === 'web') {
                openWebSearch(query);
                input.value = '';
            }
        });

        document.addEventListener('amud:category-filter', function () {
            if (getMode() === 'apps' && !input.value.trim()) {
                applyAppFilter('');
            }
        });

        initSearchToggle();

        globalThis.amudExpandAppSearch = expandSearchBar;
        globalThis.amudCollapseAppSearch = collapseSearchBar;
        globalThis.amudFocusAppSearch = function () {
            if (isMobileSearchCollapsed()) {
                expandSearchBar();
            }
            if (getMode() !== 'apps') setMode('apps');
            input.focus();
            input.select();
        };
        globalThis.amudClearAppSearch = function () {
            input.value = '';
            applyAppFilter('');
            if (globalThis.matchMedia('(max-width: 768px)').matches) {
                collapseSearchBar();
            }
        };
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
