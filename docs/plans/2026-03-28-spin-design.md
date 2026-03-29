# Spin — Design Document

A local development orchestrator that replaces Docker Compose. Spin natively launches applications and their dependency graphs on the host, with automatic resource allocation to avoid collisions.

## Core Concepts

**Two kinds of things:** resources and workloads, both backed by types.

- **Resource** — a static value: a file path, IP address/port, env var, config file, credentials. Resolved and allocated, but no execution lifecycle.
- **Workload** — something with an execution lifecycle and state: a database, REST API, application. Started, health-checked, supervised, torn down.

Whether a type is a resource or workload is controlled by an attribute: `#[resource]` or `#[workload]`. From the perspective of static analysis (type checking, constraint matching), resources and workloads are treated identically. The distinction matters only at runtime — workloads implement lifecycle hooks (start, readiness, teardown) while resources do not.

A workload's fields are themselves resources — once running, a Postgres workload *provides* a host, port, and credentials as resources for consumers.

**Three roles a `.spin` module can play:**

1. **Type definition** — an abstract contract that defines the terminology (fields and types) consumers use to express constraints and providers use to express capabilities. Think of it as a trait/interface. (e.g., `postgres.spin` defines what "Postgres" means.)

2. **Provider** — supplies instantiated resources or workloads via a concrete strategy. A provider implements a subset of the type definition's fields — partial support is fine as long as no consumer in the current graph constrains an unsupported field. (e.g., `postgres-docker.spin` supplies a running Postgres using Docker.)

3. **Composition** — declares dependencies on resources and workloads (with constraints), wires them together. This is what an "application" typically is. (e.g., `my-app.spin`.)

A single module can play multiple roles and can export multiple type definitions, providers, and compositions.

**`spin-core` provides primitive types** that everything bottoms out to. These are baked into the `spin` binary, not stored on disk. Built-in modules are named `spin-core-*`. User-defined modules cannot use the `spin-` prefix.

**Module resolution** works via `SPIN_PATH` — an ordered list of directories searched for `<name>.spin` files.

## Type System & Consumer/Provider Matching

**Data types:** primitives from `spin-core`, structs, enums, `Option<T>`, `Result<T>`.

**Type definitions are traits.** They declare a set of typed fields that form the vocabulary for a resource or workload:

```
type Postgres =
  version: spin-core::Semver,
  tls: Option<Self::Tls>,
  host: spin-core::String,
  port: spin-core::TcpPort,
  username: spin-core::String,
  password: spin-core::String,
  init_script: Option<spin-core::FilePath>;

type Tls =
  port: spin-core::TcpPort,
  ssl_key_file: spin-core::TlsKeyFile,
  ssl_cert_file: spin-core::TlsCertFile;
```

**Types compose recursively.** A type's fields can themselves be resources or workloads (e.g., Postgres TLS contains a port and key files). The entire graph resolves down to `spin-core` primitives.

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
- A module can export multiple type definitions, providers, and compositions

## Runtime Architecture

**Supervision tree.** `spin up` launches a daemonized root supervisor (`spin plumbing:supervise`) that manages the full dependency graph.

- The root supervisor spawns child supervisors for each workload in topological order (leaves first)
- Each supervisor manages its workload's lifecycle: start, readiness check, monitoring, teardown
- Resources are resolved/allocated but not supervised (they have no lifecycle)
- `spin up` connects to the root supervisor, waits for all readiness signals, then returns control to the user
- Communication between the `spin` CLI and supervisors via Unix socket

**Readiness.** Workload type definitions declare a readiness contract (what readiness looks like). Providers implement the readiness check (how to verify it).

**Teardown.** `spin down` tells the root supervisor to tear down workloads in reverse topological order. Each workload uses its provider-defined teardown if available, otherwise SIGTERM with a timeout followed by SIGKILL.

**Failure.** If a workload crashes, spin does not restart it. It reports the failure and tears down all workloads that were successfully launched.

**Resource allocation.** Resources like TCP ports are typically allocated dynamically by spin, though they can be constrained if needed. This means multiple independent `spin up` invocations (e.g., two worktrees of the same app) have zero resource collisions by default.

**Sharing in a composition.** When a parent module composes two applications that both need Postgres, the parent wires a single Postgres workload into both — the sharing is explicit at the composition level, not implicit.

**Linking across supervisor trees.** Independent supervisor trees are fully isolated by default. `spin up --link <peer-unix-socket> --use customer-database` opts in to reusing a workload from another running supervisor.

## CLI Design

**Porcelain commands** (user-facing):
- `spin up` — bring up the app and all dependencies
- `spin down` — tear it all down
- `spin tree` — query the root supervisor, display the dependency graph and status
- `spin query <expr>` — inspect resolved values from the live graph

**Plumbing commands** (`spin plumbing:*`) are implementation details. Not shown in `spin --help`, only visible via `spin --help --plumbing`. Examples:
- `spin plumbing:supervise <workload>` — launch and monitor a workload
- `spin plumbing:kill <workload>` — tear down a workload

## Static Analysis

Before anything launches, spin performs static verification:

- All module imports resolve via `SPIN_PATH`
- The dependency graph is a DAG (no cycles)
- All consumer-constrained fields are supported by the chosen provider
- Type checking — constraints match the declared field types
- Default provider is specified and resolvable for every required resource or workload

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
