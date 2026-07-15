# Production hardening

## Dedicated S3 cache

Before production, replace the shared Valkey deployment with a dedicated `s3-cache` instance:

- Attach only the Control Plane to its internal Docker network. API and Storage continue to call the Control Plane and never connect to the cache.
- Do not publish a host port or persist a volume, append-only log, or snapshots. The cache is rebuilt from OpenBao after restart.
- Give the Control Plane a dedicated Valkey ACL user limited to the S3 cache key prefixes and the commands it needs: `GET`, `SETEX`, `DEL`, and `INCR`.
- Supply the ACL credential through the deployment secret mechanism and rotate it.
- Use TLS/mTLS for Control Plane-to-cache traffic when services run across hosts or an untrusted network.

This confines cached plaintext to the dedicated cache and Control Plane memory. Do not add OpenBao Transit encryption to the cache unless compliance requires ciphertext at rest: decrypting each cache hit would make OpenBao part of the normal S3 request path again.
