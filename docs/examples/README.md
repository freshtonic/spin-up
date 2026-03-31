# Gaps

1. port: 0 for dynamic allocation — there's no formal way to say "spin, pick a port for me." We need either a special value or an allocator syntax. 0 is a convention (like binding to port 0 in TCP) but spin needs to know this means "allocate."
2. No Lifecycle integration — the types define what to run but not how. DockerEqlPostgres needs to express: "run this Docker container with these args, wait for pg_isready, then run these SQL scripts." LocalCts needs to express: "run cargo run --bin cts-server in this directory with these env vars, wait for GET /health to return 200."
3. No env var injection syntax — CTS/ZeroKMS/Proxy are all configured via env vars. We need a way to map type fields to environment variables for the launched process.
4. No Docker container type — we need a spin-docker module with a container type: image, ports, volumes, health check, env vars.
5. No HTTP health check primitive — readiness checks like "HTTP GET /health returns 200" need a built-in.
6. No SQL script execution primitive — EQL installation requires running SQL against Postgres after it's ready.
7. No cargo run workload type — launching a Rust binary from source needs a built-in: working directory, binary name, env vars, health check.
8. Cross-references between ports — cts.port is used in zerokms.cts_issuer as part of a URL string. String interpolation handles this, but the dependency is implicit rather than typed.
9. No secret management — credentials like client_key are strings. No distinction between secrets and config.