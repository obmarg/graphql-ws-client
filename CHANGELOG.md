# Changelog

All notable changes to this project will be documented in this file.

The format is roughly based on [Keep a
Changelog](http://keepachangelog.com/en/1.0.0/).

This project intends to inhere to [Semantic
Versioning](http://semver.org/spec/v2.0.0.html), but has not yet reached 1.0 so
all APIs might be changed.

## Unreleased - xxxx-xx-xx

### New Features

- `graphql-ws-client` now has an example
- `streaming_operation` now returns a `SubscriptionStream` type. This is still
  a `Stream` but also exposes a `stop_operation` function that can be used to tell
  the server to end the stream.

## v0.1.0 - 2021-04-04

- Initial release