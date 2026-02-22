<div align="center">
  <h1>GraphQL Websocket Client</h1>

  <p>
    <strong>Runtime agnostic graphql websocket client</strong>
  </p>

  <p>
    <a href="https://crates.io/crates/graphql-ws-client"><img alt="Crate Info" src="https://img.shields.io/crates/v/graphql-ws-client.svg"/></a>
    <a href="https://docs.rs/graphql-ws-client/"><img alt="API Docs" src="https://img.shields.io/docsrs/graphql-ws-client"/></a>
    <a href="https://web.libera.chat/#cynic"><img alt="IRC Room" src="https://img.shields.io/badge/Chat%20Room-5555FF"/></a>
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

### Tungstenite Versions

As the tungstenite library is pre-1.0 `graphql-ws-client` provides support for
a range of versions. You can select which version of tungestenite you want
using the `tungstenite-0-xx` feature flags. Note that only one of these can be
active at any time, or `graphql-ws-client` won't compile. Because of these
limitations only one tungstenite version will be tested on the
`grapqhl-ws-client` CI, as a result the other versions may not compile or
work corectly.

## Integrations

The library offers integrations with some popular GraphQL clients with feature flags:

- [graphql-client](https://github.com/graphql-rust/graphql-client): `features = ["client-graphql-client"]`
- [cynic](https://github.com/obmarg/cynic): `features = ["client-cynic"]`

## Documentation

The documentation is quite limited at the moment, here are some sources:

1. The provided [examples](https://github.com/obmarg/graphql-ws-client/tree/main/examples/examples)
2. The reference documentation on [docs.rs](https://docs.rs/graphql-ws-client)

## Logging

By default, the library will log some messages at the `trace` level to help you debug.
It is possible to turn off the logging entirely by using the `no-logging` feature.

## Getting Help

If you want help with graphql-ws-client you can join the #cynic chat room on
IRC using any of these options:

- [A Web UI](https://web.libera.chat/#cynic)
- [Using this link if you have an IRC client](ircs://irc.libera.chat:6697/#cynic)
- [Selecting a client from joinrc.at](https://joinirc.at/ircs://irc.libera.chat:6697/#cynic)
- Joining #cynic on irc.libera.chat if you know what you're doing
