<div align="center">
  <h1>Graphql Websocket Client</h1>

  <p>
    <strong>Runtime agnostic graphql websocket client</strong>
  </p>

  <p>
    <a href="https://crates.io/crates/graphql-ws-client"><img alt="Crate Info" src="https://img.shields.io/crates/v/graphql-ws-client.svg"/></a>
    <a href="https://docs.rs/graphql-ws-client/"><img alt="API Docs" src="https://img.shields.io/docsrs/graphql-ws-client"/></a>
    <a href="https://discord.gg/Y5xDmDP"><img alt="Discord Chat" src="https://img.shields.io/discord/754633560933269544"/></a>
  </p>

  <h4>
    <a href="https://github.com/obmarg/graphql-ws-client/tree/master/examples/examples">Examples</a>
    <span> | </span>
    <a href="https://github.com/obmarg/graphql-ws-client/blob/master/CHANGELOG.md">Changelog</a>
  </h4>
</div>

# Overview

The goal of this library is to provide a runtime agnostic implementation for [GraphQL-over-Websockets](https://github.com/graphql/graphql-over-http/blob/main/rfcs/GraphQLOverWebSocket.md).
The library only supports subscriptions for now but will eventually support queries and mutations. 
It supports the websocket libraries [async-tungstenite](https://github.com/sdroege/async-tungstenite) and [ws-stream-wasm](https://github.com/najamelan/ws_stream_wasm).

## Integrations

The library offers integrations with some popular GraphQL clients with feature flags:

- [graphql-client](https://github.com/graphql-rust/graphql-client): `features = ["client-graphql-client"]`
- [cynic](https://github.com/obmarg/cynic): `features = ["client-cynic"]`

## Documentation

The documentation is quite limited at the moment, here are some sources:

1. The provided [examples](https://github.com/obmarg/graphql-ws-client/tree/master/examples/examples)
2. The reference documentation on [docs.rs](https://docs.rs/graphql-ws-client/)

## Logging

By default, the library will log some messages at the `trace` level to help you debug.
It is possible to turn off the logging entirely by using the `no-logging` feature.
