const CACHE_NAME = 'amud-dashboard-v51';
const ASSETS_TO_CACHE = [
  '/static/style.css',
  '/static/theme-guards.css',
  '/static/theme-engine.js',
  '/static/theme-picker.js',
  '/static/AMUD-logo.png',
  '/static/pwa-icon-192.png',
  '/static/pwa-icon-512.png',
  '/static/offline.html',
  '/static/pwa-install.js',
  '/static/themes/manifest.json',
  '/static/themes/_shared.css',
  '/static/vendor/alpine.min.js',
  '/static/vendor/lucide.min.js',
  '/static/vendor/three.min.js',
  '/static/themes/backgrounds/taghawsa-bg.js',
  '/static/admin.js',
  '/static/dashboard-live.js',
  '/static/theme-scheduler.js',
  '/static/app-search.js',
  '/static/shortcuts.js',
  '/static/embed-tabs.js',
  '/static/drag.js',
  '/static/logo-picker.js'
];

const LAYOUT_CSS_PATHS = new Set([
  '/static/style.css',
  '/static/theme-guards.css',
]);

function parseManifestJson(res) {
  if (!res.ok) return null;
  return res.json();
}

function cacheThemePreviews() {
  return fetch('/static/themes/manifest.json', { cache: 'no-store' })
    .then(parseManifestJson)
    .then(function (manifest) {
      if (!manifest?.themes) return [];
      return manifest.themes
        .map(function (t) { return t.preview; })
        .filter(function (p) { return p?.startsWith('/'); });
    })
    .catch(function () { return []; });
}

function isLayoutCss(pathname) {
  return LAYOUT_CSS_PATHS.has(pathname);
}

function hasVersionQuery(url) {
  return url.searchParams.has('v');
}

function putIfOk(cache, request, response) {
  if (response?.ok) {
    cache.put(request, response.clone());
  }
  return response;
}

function networkFirst(request) {
  return caches.open(CACHE_NAME).then(function (cache) {
    return fetch(request)
      .then(function (response) {
        return putIfOk(cache, request, response);
      })
      .catch(function () {
        return cache.match(request);
      });
  });
}

function staleWhileRevalidate(request) {
  return caches.open(CACHE_NAME).then(function (cache) {
    return cache.match(request).then(function (cached) {
      const networkFetch = fetch(request)
        .then(function (response) {
          return putIfOk(cache, request, response);
        })
        .catch(function () { return cached; });
      return cached || networkFetch;
    });
  });
}

function deleteLayoutCssEntries(cache) {
  return cache.keys().then(function (requests) {
    const layoutRequests = requests.filter(function (req) {
      return isLayoutCss(new URL(req.url).pathname);
    });
    return Promise.all(layoutRequests.map(function (req) {
      return cache.delete(req);
    }));
  });
}

function purgeLayoutCssFromAllCaches() {
  return caches.keys().then(function (keys) {
    return Promise.all(keys.map(function (key) {
      return caches.open(key).then(deleteLayoutCssEntries);
    }));
  });
}

globalThis.addEventListener('install', event => {
  event.waitUntil(
    cacheThemePreviews()
      .then(function (previews) {
        return caches.open(CACHE_NAME).then(function (cache) {
          return cache.addAll(ASSETS_TO_CACHE.concat(previews.slice(0, 40)));
        });
      })
      .then(() => globalThis.skipWaiting())
  );
});

globalThis.addEventListener('activate', event => {
  event.waitUntil(
    purgeLayoutCssFromAllCaches()
      .then(function () {
        return caches.keys().then(function (keys) {
          return Promise.all(
            keys
              .filter(function (key) { return key !== CACHE_NAME; })
              .map(function (key) { return caches.delete(key); })
          );
        });
      })
      .then(function () { return globalThis.clients.claim(); })
  );
});

globalThis.addEventListener('fetch', event => {
  const request = event.request;
  const url = new URL(request.url);

  if (url.pathname.startsWith('/ws') || url.pathname.startsWith('/api/')) {
    return;
  }

  if (request.mode === 'navigate') {
    event.respondWith(
      fetch(request).catch(() =>
        caches.match('/static/offline.html').then((cached) => cached || Response.error())
      )
    );
    return;
  }

  if (url.pathname.startsWith('/static/')) {
    if (isLayoutCss(url.pathname)) {
      if (hasVersionQuery(url)) {
        event.respondWith(networkFirst(request));
      } else {
        event.respondWith(staleWhileRevalidate(request));
      }
      return;
    }
    event.respondWith(
      caches.match(request).then(cached => {
        return cached || fetch(request).then(response => {
          const copy = response.clone();
          caches.open(CACHE_NAME).then(cache => cache.put(request, copy));
          return response;
        });
      })
    );
  }
});
