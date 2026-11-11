/* Opt-in display translation. Form values and application content are never rewritten. */
(function () {
    'use strict';

    function readConfig() {
        const element = document.getElementById('amud-i18n-config');
        try {
            return JSON.parse(element ? element.textContent : '{}');
        } catch (_) {
            return {};
        }
    }

    const config = readConfig();
    const language = config.language === 'fr' ? 'fr' : 'en';
    const labels = language === 'fr' ? (config.messages || {}) : {};
    const lowercaseLabels = new Map(Object.entries(labels).map(([key, value]) => [key.toLowerCase(), value]));
    const hasLabel = key => Object.prototype.hasOwnProperty.call(labels, key);

    function t(text) {
        if (hasLabel(text)) return labels[text];
        return lowercaseLabels.get(String(text).toLowerCase()) || text;
    }

    window.amudI18n = { language, locale: language === 'fr' ? 'fr-FR' : 'en-US', t };
    if (language !== 'fr') return;

    const TRANSLATED_ATTRIBUTES = ['placeholder', 'title', 'aria-label'];

    // User content (app, feed, category, device and node names) is never rewritten.
    const SKIPPED = [
        'script', 'style', 'code', 'pre', 'textarea', '[contenteditable="true"]',
        '.app-card-title', '.app-card-desc', '.feed-card-title', '.rss-feed-title',
        '.feed-category-pill', '.wol-name', '.node-chip-name', '.dashboard-widgets-row',
        '.username-highlight', '[data-i18n-skip]',
    ].join(',');

    const UNITS = { GB: 'Go', TB: 'To', MB: 'Mo' };
    const UNIT_STATES = { Used: 'utilisés', Total: 'au total', Free: 'libres' };
    const SIZE_PATTERN = /(\d+(?:\.\d+)?)\s*(GB|TB|MB)(?:\s+(Used|Total|Free))?\b/g;
    const WEATHER_PATTERN = /\b(Partly cloudy|Overcast|Cloudy|Clear|Rain|Snow|Offline)\b/gi;

    // Category tabs and section titles hold user categories; only the built-in "All" is translated.
    function isUserCategory(element) {
        const category = element.closest('.filter-tab,.category-section-title');
        if (!category) return false;
        const isAllTab = (category.getAttribute('@click') || '').includes("filterCategory('all'");
        const isAllSection = category.parentElement?.getAttribute('data-filter-section') === 'all';
        return !isAllTab && !isAllSection;
    }

    function localiseSizes(text) {
        return text
            .replace(SIZE_PATTERN, (_, number, unit, state) =>
                number.replace('.', ',') + ' ' + UNITS[unit] + (state ? ' ' + UNIT_STATES[state] : ''))
            .replace(/\bB\/s\b/g, 'o/s');
    }

    // data-i18n-context="preset" tries "preset:Light" first, so one English word can have two translations.
    function lookup(element, key) {
        const context = element.closest('[data-i18n-context]')?.dataset.i18nContext;
        const contextKey = context ? context + ':' + key : '';
        return contextKey && hasLabel(contextKey) ? labels[contextKey] : t(key);
    }

    function translateText(node) {
        const parent = node.parentElement;
        if (!parent || parent.closest(SKIPPED) || isUserCategory(parent)) return;
        if (parent.tagName === 'OPTION' && !parent.hasAttribute('value')) return;

        let text = node.nodeValue
            .replace(/\bSign Out(?=\s*\()/g, 'Se déconnecter')
            .replace(/\bAgent offline\b/g, 'Agent hors ligne');
        if (parent.closest('.telemetry-row,.metric-value')) text = localiseSizes(text);
        if (text !== node.nodeValue) node.nodeValue = text;

        const key = text.replace(/\s+/g, ' ').trim();
        const translation = lookup(parent, key);
        if (translation && translation !== key) {
            node.nodeValue = text.replace(/\S[\s\S]*\S|\S/, translation);
        } else if (parent.id === 'live-weather') {
            const localised = text.replace(WEATHER_PATTERN, word => t(word));
            if (localised !== text) node.nodeValue = localised;
        }
    }

    function translateAttributes(element) {
        if (element.closest(SKIPPED)) return;
        for (const attribute of TRANSLATED_ATTRIBUTES) {
            const value = element.getAttribute(attribute);
            if (hasLabel(value)) element.setAttribute(attribute, labels[value]);
        }
    }

    function translate(root) {
        if (root.nodeType === Node.TEXT_NODE) {
            translateText(root);
            return;
        }
        if (root.nodeType !== Node.ELEMENT_NODE || root.matches(SKIPPED)) return;

        const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
        while (walker.nextNode()) translateText(walker.currentNode);

        translateAttributes(root);
        root.querySelectorAll('[placeholder],[title],[aria-label]').forEach(element => translateAttributes(element));
    }

    function init() {
        translate(document.body);

        const username = document.getElementById('username-display');
        if (username && username.dataset.i18nGuest === 'true') username.textContent = 'invité';

        new MutationObserver(records => {
            for (const record of records) {
                if (record.type === 'characterData') translateText(record.target);
                else if (record.type === 'attributes') translate(record.target);
                else record.addedNodes.forEach(node => translate(node));
            }
        }).observe(document.body, {
            subtree: true,
            childList: true,
            characterData: true,
            attributes: true,
            attributeFilter: TRANSLATED_ATTRIBUTES,
        });
    }

    document.documentElement.lang = 'fr';
    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
    else init();
})();
