<div align="center">
  <h1>GraphQL Websocket Client</h1>

  <p>
    <strong>Runtime agnostic graphql websocket client</strong>
  </p>

  <p>
    <a href="https://crates.io/crates/graphql-ws-client"><img alt="Crate Info" src="https://img.shields.io/crates/v/graphql-ws-client.svg?logo=rust&color=FDC452"/></a>
    <a href="https://github.com/obmarg/graphql-ws-client/actions/workflows/release.yaml"><img alt="Release Info" src="https://github.com/obmarg/graphql-ws-client/actions/workflows/release.yaml/badge.svg"/></a>
    <a href="https://docs.rs/graphql-ws-client"><img alt="API Docs" src="https://img.shields.io/docsrs/graphql-ws-client?logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCA1NzYgNTEyIj48IS0tISBGb250IEF3ZXNvbWUgRnJlZSA2LjIuMCBieSBAZm9udGF3ZXNvbWUgLSBodHRwczovL2ZvbnRhd2Vzb21lLmNvbSBMaWNlbnNlIC0gaHR0cHM6Ly9mb250YXdlc29tZS5jb20vbGljZW5zZS9mcmVlIChJY29uczogQ0MgQlkgNC4wLCBGb250czogU0lMIE9GTCAxLjEsIENvZGU6IE1JVCBMaWNlbnNlKSBDb3B5cmlnaHQgMjAyMiBGb250aWNvbnMsIEluYy4gLS0+PHBhdGggZD0iTTI5MC44IDQ4LjZsNzguNCAyOS43TDI4OCAxMDkuNSAyMDYuOCA3OC4zbDc4LjQtMjkuN2MxLjgtLjcgMy44LS43IDUuNyAwek0xMzYgOTIuNVYyMDQuN2MtMS4zIC40LTIuNiAuOC0zLjkgMS4zbC05NiAzNi40QzE0LjQgMjUwLjYgMCAyNzEuNSAwIDI5NC43VjQxMy45YzAgMjIuMiAxMy4xIDQyLjMgMzMuNSA1MS4zbDk2IDQyLjJjMTQuNCA2LjMgMzAuNyA2LjMgNDUuMSAwTDI4OCA0NTcuNWwxMTMuNSA0OS45YzE0LjQgNi4zIDMwLjcgNi4zIDQ1LjEgMGw5Ni00Mi4yYzIwLjMtOC45IDMzLjUtMjkuMSAzMy41LTUxLjNWMjk0LjdjMC0yMy4zLTE0LjQtNDQuMS0zNi4xLTUyLjRsLTk2LTM2LjRjLTEuMy0uNS0yLjYtLjktMy45LTEuM1Y5Mi41YzAtMjMuMy0xNC40LTQ0LjEtMzYuMS01Mi40bC05Ni0zNi40Yy0xMi44LTQuOC0yNi45LTQuOC0zOS43IDBsLTk2IDM2LjRDMTUwLjQgNDguNCAxMzYgNjkuMyAxMzYgOTIuNXpNMzkyIDIxMC42bC04Mi40IDMxLjJWMTUyLjZMMzkyIDEyMXY4OS42ek0xNTQuOCAyNTAuOWw3OC40IDI5LjdMMTUyIDMxMS43IDcwLjggMjgwLjZsNzguNC0yOS43YzEuOC0uNyAzLjgtLjcgNS43IDB6bTE4LjggMjA0LjRWMzU0LjhMMjU2IDMyMy4ydjk1LjlsLTgyLjQgMzYuMnpNNDIxLjIgMjUwLjljMS44LS43IDMuOC0uNyA1LjcgMGw3OC40IDI5LjdMNDI0IDMxMS43bC04MS4yLTMxLjEgNzguNC0yOS43ek01MjMuMiA0MjEuMmwtNzcuNiAzNC4xVjM1NC44TDUyOCAzMjMuMnY5MC43YzAgMy4yLTEuOSA2LTQuOCA3LjN6Ij48L3BhdGg+PC9zdmc+"/></a>
    <a href="https://discord.gg/Y5xDmDP"><img alt="Discord Chat" src="https://img.shields.io/discord/754633560933269544?logo=discord&color=%237289da"/></a>
  </p>

  <h4>
    <a href="https://github.com/obmarg/graphql-ws-client/tree/main/examples/examples">Examples</a>
    <span> | </span>
    <a href="https://github.com/obmarg/graphql-ws-client/blob/main/CHANGELOG.md">Changelog</a>
  </h4>
</div>

# Overview

The goal of this library is to provide a runtime agnostic implementation for
[GraphQL-over-Websockets](https://github.com/graphql/graphql-over-http/blob/main/rfcs/GraphQLOverWebSocket.md).

The library only supports subscriptions for now but will eventually support queries and mutations.

It supports the websocket libraries
[async-tungstenite](https://github.com/sdroege/async-tungstenite),
[tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) and
[ws-stream-wasm](https://github.com/najamelan/ws_stream_wasm), and

## Integrations

The library offers integrations with some popular GraphQL clients with feature flags:

-   [graphql-client](https://github.com/graphql-rust/graphql-client): `features = ["client-graphql-client"]`
-   [cynic](https://github.com/obmarg/cynic): `features = ["client-cynic"]`

## Documentation

The documentation is quite limited at the moment, here are some sources:

1. The provided [examples](https://github.com/obmarg/graphql-ws-client/tree/main/examples/examples)
2. The reference documentation on [docs.rs](https://docs.rs/graphql-ws-client)

## Logging

By default, the library will log some messages at the `trace` level to help you debug.
It is possible to turn off the logging entirely by using the `no-logging` feature.
