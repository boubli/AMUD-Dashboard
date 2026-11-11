/* Opt-in display translation. Form values and application content are never rewritten. */
(function () {
    'use strict';
    const configElement = document.getElementById('amud-i18n-config');
    let config = {};
    try { config = JSON.parse(configElement ? configElement.textContent : '{}'); } catch (_) { /* English fallback */ }
    const language = config.language === 'fr' ? 'fr' : 'en';
    const labels = language === 'fr' ? (config.messages || {}) : {};
    const folded = new Map(Object.entries(labels).map(([key, value]) => [key.toLowerCase(), value]));
    const t = text => Object.prototype.hasOwnProperty.call(labels, text) ? labels[text] : (folded.get(String(text).toLowerCase()) || text);
    window.amudI18n = {language, locale: language === 'fr' ? 'fr-FR' : 'en-US', t};
    if (language !== 'fr') return;
    const attributes = ['placeholder', 'title', 'aria-label'];
    // User-authored content (app, feed, category, device and node names) must never be rewritten.
    const ignored = 'script,style,code,pre,textarea,[contenteditable="true"],.app-name,.app-description,.app-card-title,.app-card-desc,.feed-card-title,.rss-feed-title,.feed-category-pill,.wol-name,.node-chip-name,.dashboard-widgets-row,.username-highlight,[data-i18n-skip]';
    const normalise = text => text.replace(/\s+/g, ' ').trim();
    function translateText(node) {
        if (!node.parentElement || node.parentElement.closest(ignored)) return;
        const category = node.parentElement.closest('.filter-tab,.category-section-title');
        if (category && !(category.getAttribute('@click') || '').includes("filterCategory('all'")
            && category.parentElement?.getAttribute('data-filter-section') !== 'all') return;
        if (node.parentElement.tagName === 'OPTION' && !node.parentElement.hasAttribute('value')) return;
        let text = node.nodeValue;
        text = text.replace(/\bSign Out(?=\s*\()/g, 'Se déconnecter');
        text = text.replace(/\bAgent offline\b/g, 'Agent hors ligne');
        if (node.parentElement.closest('.telemetry-row,.metric-value')) {
            text = text.replace(/(\d+(?:\.\d+)?)\s*(GB|TB|MB)(?:\s+(Used|Total|Free))?\b/g, (_, number, unit, state) =>
                number.replace('.', ',') + ' ' + ({GB: 'Go', TB: 'To', MB: 'Mo'}[unit]) + (state ? ' ' + ({Used: 'utilisés', Total: 'au total', Free: 'libres'}[state]) : ''));
            text = text.replace(/\bB\/s\b/g, 'o/s');
        }
        if (text !== node.nodeValue) node.nodeValue = text;
        const key = normalise(node.nodeValue);
        // data-i18n-context="preset" looks up "preset:Light" first, so one English word can have two meanings.
        const context = node.parentElement.closest('[data-i18n-context]')?.dataset.i18nContext;
        const contextKey = context ? context + ':' + key : '';
        const translation = contextKey && Object.prototype.hasOwnProperty.call(labels, contextKey) ? labels[contextKey] : t(key);
        if (translation && translation !== key) {
            node.nodeValue = node.nodeValue.replace(/\S[\s\S]*\S|\S/, translation);
        } else if (node.parentElement.id === 'live-weather') {
            const next = node.nodeValue.replace(/\b(Partly cloudy|Overcast|Cloudy|Clear|Rain|Snow|Offline)\b/gi, value => {
                const key = Object.keys(labels).find(key => key.toLowerCase() === value.toLowerCase());
                return key ? labels[key] : value;
            });
            if (next !== node.nodeValue) node.nodeValue = next;
        }
    }
    function translate(root) {
        if (root.nodeType === Node.TEXT_NODE) return translateText(root);
        if (root.nodeType !== Node.ELEMENT_NODE) return;
        if (root.matches(ignored)) return;
        const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
        while (walker.nextNode()) translateText(walker.currentNode);
        [root, ...root.querySelectorAll('[placeholder],[title],[aria-label]')].forEach(element => {
            if (element.closest(ignored)) return;
            attributes.forEach(attribute => {
                const value = element.getAttribute(attribute);
                if (Object.prototype.hasOwnProperty.call(labels, value)) element.setAttribute(attribute, labels[value]);
            });
        });
    }
    document.documentElement.lang = 'fr';
    function init() {
        translate(document.body);
        const username = document.getElementById('username-display');
        if (username && username.dataset.i18nGuest === 'true') username.textContent = 'invité';
        new MutationObserver(records => {
            for (const record of records) {
                if (record.type === 'characterData') translateText(record.target);
                else if (record.type === 'attributes') translate(record.target);
                else for (const node of record.addedNodes) translate(node);
            }
        }).observe(document.body, {subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: attributes});
    }
    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
    else init();
})();
