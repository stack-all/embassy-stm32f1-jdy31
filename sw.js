// Install event - cache resources
self.addEventListener('install', event => {
    console.log('Service Worker installing...');
    event.waitUntil(
        caches.open(CACHE_NAME)
            .then(cache => {
                console.log('Caching app shell and resources');
                return cache.addAll(urlsToCache);
            })
            .catch(error => {
                console.error('Caching failed:', error);
                // Continue installing even if some resources fail to cache
                return caches.open(CACHE_NAME)
                    .then(cache => {
                        // Cache resources one by one, ignoring failures
                        const cachePromises = urlsToCache.map(url => {
                            return cache.add(url).catch(err => {
                                console.warn(`Failed to cache ${url}:`, err);
                            });
                        });
                        return Promise.allSettled(cachePromises);
                    });
            })
    );
    self.skipWaiting();
});

// Activate event - clean up old caches
self.addEventListener('activate', event => {
    console.log('Service Worker activating...');
    event.waitUntil(
        caches.keys().then(cacheNames => {
            return Promise.all(
                cacheNames.map(cacheName => {
                    if (cacheName !== CACHE_NAME) {
                        console.log('Deleting old cache:', cacheName);
                        return caches.delete(cacheName);
                    }
                })
            );
        })
    );
    self.clients.claim();
});

// Fetch event - serve cached content when offline
self.addEventListener('fetch', event => {
    // Skip non-GET requests
    if (event.request.method !== 'GET') {
        return;
    }

    // Skip chrome-extension requests
    if (event.request.url.startsWith('chrome-extension://')) {
        return;
    }

    event.respondWith(
        caches.match(event.request)
            .then(response => {
                // Return cached version if available
                if (response) {
                    console.log('Serving from cache:', event.request.url);
                    return response;
                }

                // Otherwise, fetch from network
                return fetch(event.request)
                    .then(response => {
                        // Don't cache non-successful responses
                        if (!response || response.status !== 200 || response.type !== 'basic') {
                            return response;
                        }

                        // Clone the response for caching
                        const responseToCache = response.clone();

                        // Cache successful responses for future use
                        caches.open(CACHE_NAME)
                            .then(cache => {
                                // Only cache certain types of requests
                                const url = event.request.url;
                                if (url.includes('unpkg.com') || 
                                    url.includes('fonts.bunny.net') || 
                                    url.endsWith('.html') ||
                                    url.endsWith('.js') ||
                                    url.endsWith('.css')) {
                                    cache.put(event.request, responseToCache);
                                }
                            });

                        return response;
                    })
                    .catch(error => {
                        console.log('Network request failed, serving fallback:', error);
                        
                        // For HTML requests, return the cached index.html
                        if (event.request.headers.get('accept').includes('text/html')) {
                            return caches.match('./index.html');
                        }
                        
                        // For other requests, just fail
                        throw error;
                    });
            })
    );
});

// Handle messages from the main thread
self.addEventListener('message', event => {
    if (event.data && event.data.type === 'SKIP_WAITING') {
        self.skipWaiting();
    }
});
