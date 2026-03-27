# Spin — Design Document

A local development orchestrator that replaces Docker Compose. Spin natively launches applications and their dependency graphs on the host, with automatic resource allocation to avoid collisions.

## Core Concepts

**Everything is a resource.** Applications, databases, TLS certificates, TCP ports — all modelled uniformly. The difference is only in what a resource does internally (allocate a port, launch a process, generate a file).

**Three roles a `.spin` module can play:**

1. **Resource definition** — an abstract contract that defines the terminology (fields and types) consumers use to express constraints and providers use to express capabilities. Think of it as a trait/interface. (e.g., `postgres.spin` defines what "Postgres" means.)

2. **Provider** — supplies instantiated resources via a concrete strategy. A provider implements a subset of the resource definition's fields — partial support is fine as long as no consumer in the current graph constrains an unsupported field. (e.g., `postgres-docker.spin` supplies a running Postgres using Docker.)

3. **Composition** — declares dependencies on resources (with constraints), wires them together, and optionally launches processes. This is what an "application" typically is. (e.g., `my-app.spin`.)

A single module can play multiple roles and can export multiple resource definitions, providers, and compositions.

**`spin-core` provides primitive resource types** that everything bottoms out to: `TcpPort`, `TempDir`, `TlsKeyFile`, `TlsCertFile`, `FilePath`, `String`, `Semver`, etc. These are baked into the `spin` binary, not stored on disk. Built-in modules are named `spin-core-*`. User-defined modules cannot use the `spin-` prefix.

**Module resolution** works via `SPIN_PATH` — an ordered list of directories searched for `<name>.spin` files.

## Type System & Consumer/Provider Matching

**Data types:** primitives from `spin-core`, hashmaps, sets, lists, structs, enums, `Option<T>`.

**Resource definitions are traits.** They declare a set of typed fields that form the vocabulary for that resource:

```
resource Postgres {
  version: spin-core::Semver,
  tls: Option<Self::Tls>,
  host: spin-core::String,
  port: spin-core::TcpPort,
  username: spin-core::String,
  password: spin-core::String,
  init_script: Option<spin-core::FilePath>,
}

resource Tls {
  port: spin-core::TcpPort,
  ssl_key_file: spin-core::TlsKeyFile,
  ssl_cert_file: spin-core::TlsCertFile,
}
```

**Resources compose recursively.** A resource's fields can themselves be resources (e.g., Postgres TLS contains `TcpPort`, `TlsKeyFile`). The entire graph resolves down to `spin-core` primitives.

**Consumers constrain selectively.** A consumer declares "I need a Postgres" and only puts constraints on the fields it cares about. Unconstrained fields are don't-cares. Constraints are type-appropriate (e.g., semver ranges on semver fields).

**Providers supply selectively.** A provider declares `supplies Postgres` and supports a subset of fields. A lightweight provider might not support `tls` at all — fine as long as no consumer constrains it.

**Static verification.** Before launching anything, spin checks that for every consumer-constrained field, the chosen provider supports that field. A mismatch is a static error.

## The `.spin` Language

A declarative-first DSL with enough expressiveness for real configuration, but not a general-purpose language.

**Language features:**
- String interpolation / variable references (`${postgres.host}`)
- Conditionals (`if platform == "macos" then ...`)
- Default values / fallbacks (`${env.PORT or 8080}`)
- Functions / reusable blocks
- `map` and `filter` (functional — no imperative loops)
- Structs, enums, hashmaps, sets, lists
- Type system built on `spin-core` primitives

**Module structure:**
- File name is the module name (`postgres.spin` is the `postgres` module)
- Modules import other modules by name, resolved via `SPIN_PATH`
- A module can export multiple resource definitions, providers, and compositions

## Runtime Architecture

**Supervision tree.** `spin up` launches a daemonized root supervisor (`spin plumbing:supervise`) that manages the full resource graph.

- The root supervisor spawns child supervisors for each resource in topological order (leaves first)
- Each supervisor manages its resource's lifecycle: start, readiness check, monitoring, teardown
- `spin up` connects to the root supervisor, waits for all readiness signals, then returns control to the user
- Communication between the `spin` CLI and supervisors via Unix socket

**Readiness.** Resource definitions declare a readiness contract (what readiness looks like). Providers implement the readiness check (how to verify it).

**Teardown.** `spin down` tells the root supervisor to tear down in reverse topological order. Each resource uses its provider-defined teardown if available, otherwise SIGTERM with a timeout followed by SIGKILL.

**Failure.** If a resource crashes, spin does not restart it. It reports the failure and tears down all resources that were successfully launched.

**Resource allocation.** Resources like TCP ports are typically allocated dynamically by spin, though they can be constrained if needed. This means multiple independent `spin up` invocations (e.g., two worktrees of the same app) have zero resource collisions by default.

**Shared resources in a composition.** When a parent module composes two applications that both need Postgres, the parent wires a single Postgres resource into both — the sharing is explicit at the composition level, not implicit.

**Linking across supervisor trees.** Independent supervisor trees are fully isolated by default. `spin up --link <peer-unix-socket> --use customer-database` opts in to reusing a resource from another running supervisor.

## CLI Design

**Porcelain commands** (user-facing):
- `spin up` — bring up the app and all dependencies
- `spin down` — tear it all down
- `spin tree` — query the root supervisor, display the resource graph and status
- `spin query <expr>` — inspect resolved values from the live graph

**Plumbing commands** (`spin plumbing:*`) are implementation details. Not shown in `spin --help`, only visible via `spin --help --plumbing`. Examples:
- `spin plumbing:supervise <resource>` — launch and monitor a resource
- `spin plumbing:kill <resource>` — tear down a resource

## Static Analysis

Before anything launches, spin performs static verification:

- All module imports resolve via `SPIN_PATH`
- The dependency graph is a DAG (no cycles)
- All consumer-constrained fields are supported by the chosen provider
- Type checking — constraints match the declared field types
- Default provider is specified and resolvable for every required resource

## Developer Workflow

Designed for the "clone and go" experience:

```
git clone my-app
cd my-app
spin up
```

The app's `.spin` file declares everything needed. `SPIN_PATH` points to shared module directories containing resource definitions and providers. Spin resolves the graph, allocates resources, launches everything, and reports readiness.

Applications declare how they receive configuration — env vars, config files, CLI args. Spin resolves values (allocated ports, credentials, paths) and injects them via the declared mechanism.

## Why Not Docker

- No double-compilation (host then container)
- Native debugger attachment
- Faster iteration cycles
- Dynamic resource allocation eliminates port conflict management
