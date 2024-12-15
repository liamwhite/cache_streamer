# cache_streamer

An incomplete Rust and tokio-based HTTP server designed as a caching reverse proxy for binary files (think images and videos). It is intended to handle range requests, as well as standard GET/HEAD requests.

TODO: testing, more headers, TTL-based expiration, if-modified-since, if-none-match, if-range

### Why not nginx

- proxy_cache appears to download the full file first before starting to deliver it
- Noticeable delay before stream starts for larger files
- Difficult to completely control headers without lua/openresty
- Difficult to implement access control for purging without lua/openresty

### Why not varnish

- No internal event loop. Varnish makes heavy use of OS threading to handle connections
- No support connecting to HTTPS proxy backends unless you pay enterprise money (which I don't have)
  * You can alternatively use something like stunnel, but then you need even more threads
- VCL can't really do anything useful
- VCL's lack of usefulness means useful functionality is written in inscrutable and unsafe C extensions

## Internal architecture

LRU cache of reference-counted stream objects, each of which contains a sparse mapping of reference-counted file bytes.

When called, the service checks if the requested path already exists in the cache. If it does, then the cached information is used to generate the response. Otherwise, an upstream HTTP request matching the call is made, and a new cache entry is created if the response is success (200-206). Otherwise, the upstream response status and body are passed through to the client.

During creation, each streamer object will continue to download the response body into the cache entry's sparse mapping. Clients may request ranges which have not yet been downloaded; once a section which has not yet been downloaded is encountered, a request is made at the current file offset to fetch the unfetched section and fill in the rest of the sparse mapping.
