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

The goal of this library is to provide a runtime agnostic implementation for [GraphQL-over-Websocket](https://github.com/enisdenjo/graphql-ws/blob/master/PROTOCOL.md).
This is typically used for subscriptions but it can also be used for queries and mutations if you wish.

The library is built upon the websocket library [async-tungstenite](https://github.com/sdroege/async-tungstenite) which provides a runtime agnostic websocket implementation.
It also offers integrations with some popular GraphQL clients with feature flags:

- [graphql-client](https://github.com/graphql-rust/graphql-client): `features = ["client-graphql-client"]`
- [cynic](https://github.com/obmarg/cynic): `features = ["client-cynic"]`
