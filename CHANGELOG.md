# Changelog

All notable changes to this project will be documented in this file.

The format is roughly based on [Keep a
Changelog](http://keepachangelog.com/en/1.0.0/).

This project intends to inhere to [Semantic
Versioning](http://semver.org/spec/v2.0.0.html), but has not yet reached 1.0 so
all APIs might be changed.

## Unreleased - xxxx-xx-xx

## v0.8.0-rc.1 - 2024-02-10

### Breaking Changes

- The `next` api is now available at the top level rather than the `next`
  module.
- `async_tungstenite` is no longer a default feautre, you should explicitly
  enable it if you need it
- Updated to `async_tungstenite` 0.25
- Renamed `Client::streaming_operation` to subscribe in `next` api.

### Deprecations

These will be removed in a future version, probably in v0.9.0

- `AsyncWebsocketClient` and all its supporting traits and structs are now
  deprecated.

### New Features

- Added a `subscribe` function to `next::ClientBuilder` to make
  creating a single subscription on a given connection easier.

## v0.8.0-alpha.2 - 2024-01-30

### Breaking Changes

- `Error::Close` now has a code as well as a reason.

### New Features

- Added a `next` module with a significant re-work of the API

## v0.8.0-alpha.1 - 2024-01-19

### Breaking Changes

- Subscription IDs sent to the server are now just monotonic numbers rather
  than uuids.
- `SubscriptionStream` no longer takes `GraphqlClient` as a generic parameter
- Removed the `GraphqlClient` trait and all its uses
- Changed the `StreamingOperation` trait:
  - Removed the `GenericResponse` associated type
  - `decode_response` now always takes a `serde_json::value` - I expect most
    implementations of this will now just be a call to `serde_json::from_value`

## v0.7.0 - 2024-01-03

### Breaking Changes

- `async_tungstenite` dependency has been updated to 0.24

## v0.6.0 - 2023-10-01

### Breaking Changes

- `async_tungstenite` dependency has been updated to 0.23, pulling in a tungstenite security fix

## v0.5.0 - 2023-07-13

### Breaking Changes

- `cynic` dependency has been updated to 3
- `graphql_client` dependency has been updated to 0.13
- `async_tungstenite` dependency has been updated to 0.22

### Bug Fixes

- Updated the cynic code to support operations with variables.

## v0.4.0 - 2023-04-02

### Breaking Changes

- `cynic` dependency has been updated to 2.2
- `async_tungstenite` dependency has been updated to 0.19
- `graphql_client` dependency has been updated to 0.12

### Bug Fixes

- The examples now compile.

## v0.3.0 - 2022-12-26

### New Features

- Added support for wasm with the `ws_stream_wasm` library.

### Changes

- Updated some dependency versions

### Bug Fixes

- `graphql-ws-client` will no longer panic when it receives a `Ping` event.
- The `AsyncWebsocketClientBuilder` type is now `Send`.

## v0.2.0 - 2022-01-27

### Breaking Changes

- Clients are now created through builder types rather than directly. See the
  `AsyncWebsocketClientBuilder` type (or it's `CynicClientBuilder` alias)
- `cynic` support is now behind the `client-cynic` feature.
- It's now recommended to use a custom impl of `futures::task::Spawn` for tokio
  rather than the `async_executors` crate, as `async_executors` is not
  compatible with `#[tokio::main]`. An example impl is provided for `tokio` in
  the examples folder.

### New Features

- `graphql_client` is now supported, behind the `client-graphql-client` feature.
- `graphql-ws-client` now has an example
- `streaming_operation` now returns a `SubscriptionStream` type. This is still
  a `Stream` but also exposes a `stop_operation` function that can be used to
  tell the server to end the stream.
- `cynic` no longer requires the use of `async_executors` - it now only
  requires an `impl futures::task::Spawn`. An example is included for tokio.
  Old code using the `AsyncStd` executor should continue to work but tokio
  users are encouraged to provide their own using the example.

### Bug Fixes

- `graphql-ws-client` has better support for running inside `#[tokio::main]`
- Cynic will now use the `log` crate rather than printing to stdout.

## v0.1.0 - 2021-04-04

- Initial release
