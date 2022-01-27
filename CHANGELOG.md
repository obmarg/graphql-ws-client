# Changelog

All notable changes to this project will be documented in this file.

The format is roughly based on [Keep a
Changelog](http://keepachangelog.com/en/1.0.0/).

This project intends to inhere to [Semantic
Versioning](http://semver.org/spec/v2.0.0.html), but has not yet reached 1.0 so
all APIs might be changed.

## Unreleased - xxxx-xx-xx

### Breaking Changes

- Clients are now created through builder types rather than directly.  See the
  `AsyncWebsocketClientBuilder` type (or it's `CynicClientBuilder` alias)
- `cynic` support is now behind the `client-cynic` feature.
- It's now recommended to use a custom impl of `futures::task::Spawn` for tokio
  rather than the `async_executors` crate, as `async_executors` is not
  compatible with `#[tokio::main]`.  An example impl is provided for `tokio` in
  the examples folder.


### New Features

- `graphql_client` is now supported, behind the `client-graphql-client` feature.
- `graphql-ws-client` now has an example
- `streaming_operation` now returns a `SubscriptionStream` type. This is still
  a `Stream` but also exposes a `stop_operation` function that can be used to
  tell the server to end the stream.
- `cynic` no longer requires the use of `async_executors` - it now only
  requires an `impl futures::task::Spawn`.  An example is included for tokio.
  Old code using the `AsyncStd` executor should continue to work but tokio
  users are encouraged to provide their own using the example.

### Bug Fixes

- `graphql-ws-client` has better support for running inside `#[tokio::main]`
- Cynic will now use the `log` crate rather than printing to stdout.

## v0.1.0 - 2021-04-04

- Initial release
