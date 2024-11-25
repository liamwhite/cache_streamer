# cache_streamer

An incomplete Rust and tokio-based HTTP server designed as a caching reverse proxy for binary files (think images and videos). It is intended to handle range requests, as well as standard GET/HEAD requests.

TODO: testing, more headers, TTL-based expiration, if-modified-since, if-none-match, if-range

## Why not nginx

- proxy_cache appears to download the full file first before starting to deliver it
- noticeable delay before stream starts for larger files
- difficult to completely control headers without lua/openresty
- difficult to implement access control for purging without lua/openresty

## Why not varnish

- No internal event loop. Varnish makes heavy use of OS threading to handle connections
- No support connecting to HTTPS proxy backends unless you pay enterprise money (which I don't have)
  * You can alternatively use something like stunnel, but then you need even more threads
- VCL can't really do anything useful
- VCL's lack of usefulness means useful functionality is written in inscrutable and unsafe C extensions
