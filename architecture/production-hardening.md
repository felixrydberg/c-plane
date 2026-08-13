# Production hardening

## Dedicated S3 cache

Before production, replace the shared Valkey deployment with a dedicated `s3-cache` instance:

- Attach only the API to its internal Docker network. Storage calls the API and never connects to the cache.
- Do not publish a host port or persist a volume, append-only log, or snapshots. The cache is rebuilt from OpenBao after restart.
- Give the API a dedicated Valkey ACL user limited to the S3 cache key prefixes and the commands it needs: `GET`, `SETEX`, `DEL`, and `INCR`.
- Supply the ACL credential through the deployment secret mechanism and rotate it.
- Use TLS/mTLS for API-to-cache traffic when services run across hosts or an untrusted network.

This confines cached plaintext to the dedicated cache and API memory.
