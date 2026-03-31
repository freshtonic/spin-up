// =============================================================================
// CipherStash Proxy — Local Development Environment
// =============================================================================
//
// This Spin module brings up the full CipherStash Proxy local dev stack:
//   - PostgreSQL (Docker) with EQL extension installed
//   - Local KMS (Docker)
//   - CTS (cargo run)
//   - ZeroKMS (cargo run)
//   - CipherStash Proxy (cargo run)
//
// Usage:
//   spin up proxy_dev
//
// All TCP ports are dynamically allocated — no conflicts between worktrees.
// =============================================================================

import spin-net
import spin-db-postgres
import spin-docker

// ---------------------------------------------------------------------------
// Interfaces
// ---------------------------------------------------------------------------

/// A PostgreSQL database with the EQL extension installed.
interface EqlPostgres =
  endpoint: spin-db-postgres::PostgresEndpoint,
  eql_version: string,
;

/// A local AWS KMS-compatible key management service.
interface LocalKms =
  endpoint: spin-net::SocketAddr,
;

/// CipherStash Token Server.
interface Cts =
  endpoint: spin-net::SocketAddr,
  metrics_endpoint: spin-net::SocketAddr,
  jwt_signing_key_id: string,
  workspace_crn: string,
  client_access_key: string,
;

/// ZeroKMS — key management and encryption service.
interface ZeroKms =
  endpoint: spin-net::SocketAddr,
  metrics_endpoint: spin-net::SocketAddr,
  client_id: string,
  client_key: string,
;

/// CipherStash Proxy — sits in front of Postgres.
interface CipherstashProxy =
  listen: spin-net::SocketAddr,
  metrics: spin-net::SocketAddr,
  database: EqlPostgres,
  cts: Cts,
  zerokms: ZeroKms,
;

// ---------------------------------------------------------------------------
// Types — Postgres with EQL (Docker)
// ---------------------------------------------------------------------------

/// A PostgreSQL database running in Docker with EQL installed.
type DockerEqlPostgres =
  pg_version: number,
  db_name: string,
  username: string,
  password: string,
  port: number,
  eql_version: string,
  eql_install_sql: string,
;

impl EqlPostgres for DockerEqlPostgres {
  endpoint: spin-net::SocketAddr::V4(
    ip: spin-net::IpAddrV4(octets: #[127, 0, 0, 1]),
    port: self.port,
  ),
  eql_version: self.eql_version,
}

// ---------------------------------------------------------------------------
// Types — Local KMS (Docker)
// ---------------------------------------------------------------------------

/// nsmithuk/local-kms running in Docker.
type DockerLocalKms =
  port: number,
;

impl LocalKms for DockerLocalKms {
  endpoint: spin-net::SocketAddr::V4(
    ip: spin-net::IpAddrV4(octets: #[127, 0, 0, 1]),
    port: self.port,
  ),
}

// ---------------------------------------------------------------------------
// Types — CTS (cargo run)
// ---------------------------------------------------------------------------

/// CTS launched via `cargo run` from a local checkout.
type LocalCts =
  source_dir: string,
  port: number,
  metrics_port: number,
  db_host: string,
  db_port: number,
  db_name: string,
  db_user: string,
  db_password: string,
  kms_endpoint: string,
  jwt_signing_key_id: string,
  workspace_crn: string,
  client_access_key: string,
;

impl Cts for LocalCts {
  endpoint: spin-net::SocketAddr::V4(
    ip: spin-net::IpAddrV4(octets: #[127, 0, 0, 1]),
    port: self.port,
  ),
  metrics_endpoint: spin-net::SocketAddr::V4(
    ip: spin-net::IpAddrV4(octets: #[127, 0, 0, 1]),
    port: self.metrics_port,
  ),
  jwt_signing_key_id: self.jwt_signing_key_id,
  workspace_crn: self.workspace_crn,
  client_access_key: self.client_access_key,
}

// ---------------------------------------------------------------------------
// Types — ZeroKMS (cargo run)
// ---------------------------------------------------------------------------

/// ZeroKMS launched via `cargo run` from a local checkout.
type LocalZeroKms =
  source_dir: string,
  port: number,
  metrics_port: number,
  db_host: string,
  db_port: number,
  db_name: string,
  db_user: string,
  db_password: string,
  kms_endpoint: string,
  cts_issuer: string,
  root_key_id: string,
  client_id: string,
  client_key: string,
;

impl ZeroKms for LocalZeroKms {
  endpoint: spin-net::SocketAddr::V4(
    ip: spin-net::IpAddrV4(octets: #[127, 0, 0, 1]),
    port: self.port,
  ),
  metrics_endpoint: spin-net::SocketAddr::V4(
    ip: spin-net::IpAddrV4(octets: #[127, 0, 0, 1]),
    port: self.metrics_port,
  ),
  client_id: self.client_id,
  client_key: self.client_key,
}

// ---------------------------------------------------------------------------
// Types — CipherStash Proxy (cargo run)
// ---------------------------------------------------------------------------

/// CipherStash Proxy launched via `cargo run` from a local checkout.
type LocalProxy =
  source_dir: string,
  port: number,
  metrics_port: number,
  database: EqlPostgres,
  cts: Cts,
  zerokms: ZeroKms,
;

impl CipherstashProxy for LocalProxy {
  listen: spin-net::SocketAddr::V4(
    ip: spin-net::IpAddrV4(octets: #[0, 0, 0, 1]),
    port: self.port,
  ),
  metrics: spin-net::SocketAddr::V4(
    ip: spin-net::IpAddrV4(octets: #[127, 0, 0, 1]),
    port: self.metrics_port,
  ),
  database: self.database,
  cts: self.cts,
  zerokms: self.zerokms,
}

// ---------------------------------------------------------------------------
// Wiring — the full local dev stack
// ---------------------------------------------------------------------------

// Shared Postgres for CTS, ZeroKMS, and the application database
let postgres = DockerEqlPostgres {
  pg_version: 17,
  db_name: "cipherstash",
  username: "cipherstash",
  password: "p@ssword",
  port: 0,       // dynamically allocated by spin
  eql_version: "eql-2.2.1",
  eql_install_sql: "https://github.com/cipherstash/encrypt-query-language/releases/download/${self.eql_version}/cipherstash-encrypt.sql",
}

let local_kms = DockerLocalKms {
  port: 0,       // dynamically allocated
}

let cts = LocalCts {
  source_dir: "~/cipherstash/cipherstash-suite",
  port: 0,       // dynamically allocated
  metrics_port: 0,
  db_host: "127.0.0.1",
  db_port: postgres.port,
  db_name: "cts_dev_server",
  db_user: "cipherstash",
  db_password: "p@ssword",
  kms_endpoint: "http://127.0.0.1:${local_kms.port}",
  jwt_signing_key_id: "800d5768-3fd7-4edd-a4b8-4c81c3e4c148",
  workspace_crn: "crn:cipherstash:workspace:dev",
  client_access_key: "dev-access-key",
}

let zerokms = LocalZeroKms {
  source_dir: "~/cipherstash/cipherstash-suite",
  port: 0,       // dynamically allocated
  metrics_port: 0,
  db_host: "127.0.0.1",
  db_port: postgres.port,
  db_name: "zerokms_dev",
  db_user: "cipherstash",
  db_password: "p@ssword",
  kms_endpoint: "http://127.0.0.1:${local_kms.port}",
  cts_issuer: "http://127.0.0.1:${cts.port}/",
  root_key_id: "bc436485-5092-42b8-92a3-0aa8b93536dc",
  client_id: "dev-client-id",
  client_key: "dev-client-key",
}

let proxy = LocalProxy {
  source_dir: "~/cipherstash/proxy",
  port: 0,       // dynamically allocated
  metrics_port: 0,
  database: postgres,
  cts: cts,
  zerokms: zerokms,
}

// ---------------------------------------------------------------------------
// Top-level entry point
// ---------------------------------------------------------------------------

// `spin up proxy_dev` launches everything
let proxy_dev = proxy
