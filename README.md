# 🌀Spin

Spin is a local development orchestrator that launches your applications directly on the host.

## 🚫 Docker

You know what sucks? Compiling and running your app & unit tests the compiling and running it *again* so that you can run integration tests in Docker.

You know what else sucks? Trying to debug an application running inside a Docker container.

## Stop hacking env vars

Want to run two different branches of your app and its dependencies simultaneously? Spin has your back!

All resources e.g. IP addresses, ports, files are automatically allocated from a pool so that multiple running environments never conflict with each other and no ENV var hacking is required.

## Simple overrides

Want to point part of your local stack at a production service? In Spin every definition is lazily evaluated which makes it possible to just redefine *one* `let` binding at the top-level and everything else in your stack will see the new definition.

## Language server


## VSCode extension

