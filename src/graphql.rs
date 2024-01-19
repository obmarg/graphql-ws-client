//! This module contains traits that abstract over GraphQL implementations,
//! allowing this library to be used with different GraphQL client libraries.
//!
//! Support is provided for [`cynic`][cynic] & [`graphql_client`][graphql-client],
//! but other client libraries can be added by implementing these traits for
//! those libraries.
//!
//! [cynic]: https://cynic-rs.dev
//! [graphql-client]: https://github.com/graphql-rust/graphql-client

/// A trait for GraphQL clients.
pub trait GraphqlClient {
    /// The generic response type for this GraphqlClient implementation
    ///
    /// Our client will decode this, then pass it to a `GraphqlOperation` for decoding
    /// to the specific response type of the GraphqlOperation.
    type Response: serde::de::DeserializeOwned + Send;

    /// The error that will be returned from failed attempts to decode a `Response`.
    type DecodeError: std::error::Error + Send + 'static;

    /// Decodes some error JSON into a `Response`
    fn error_response(errors: Vec<serde_json::Value>) -> Result<Self::Response, Self::DecodeError>;
}

/// An abstraction over GraphQL operations.
pub trait GraphqlOperation: serde::Serialize {
    /// The actual response & error type of this operation.
    type Response;

    /// The error that will be returned from failed attempts to decode a `Response`.
    type Error: std::error::Error;

    /// Decodes a `GenericResponse` into the actual response that will be returned
    /// to users for this operation.
    fn decode(&self, data: serde_json::Value) -> Result<Self::Response, Self::Error>;
}

#[cfg(feature = "cynic")]
pub use self::cynic::Cynic;

#[cfg(feature = "cynic")]
mod cynic {
    use super::*;

    /// Provides an implementation of [GraphqlClient] for the cynic GraphQL crate
    pub struct Cynic {}

    impl GraphqlClient for Cynic {
        type Response = ::cynic::GraphQlResponse<serde_json::Value>;

        type DecodeError = serde_json::Error;

        fn error_response(
            errors: Vec<serde_json::Value>,
        ) -> Result<Self::Response, Self::DecodeError> {
            Ok(::cynic::GraphQlResponse {
                data: None,
                errors: Some(
                    errors
                        .into_iter()
                        .map(serde_json::from_value)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            })
        }
    }

    impl<ResponseData, Variables> GraphqlOperation
        for ::cynic::StreamingOperation<ResponseData, Variables>
    where
        ResponseData: serde::de::DeserializeOwned,
        Variables: serde::Serialize,
    {
        type Response = ::cynic::GraphQlResponse<ResponseData>;

        type Error = serde_json::Error;

        fn decode(&self, response: serde_json::Value) -> Result<Self::Response, Self::Error> {
            serde_json::from_value(response)
        }
    }
}

#[cfg(feature = "client-graphql-client")]
pub use self::graphql_client::GraphQLClient;

#[cfg(feature = "client-graphql-client")]
pub use self::graphql_client::StreamingOperation;

#[cfg(feature = "client-graphql-client")]
mod graphql_client {
    use super::*;
    use ::graphql_client::{GraphQLQuery, QueryBody, Response};
    use std::marker::PhantomData;

    /// Provides an implementation of [GraphqlClient] for the graphql_client GraphQL crate
    pub struct GraphQLClient {}

    impl GraphqlClient for GraphQLClient {
        type Response = Response<serde_json::Value>;

        type DecodeError = serde_json::Error;

        fn error_response(
            errors: Vec<serde_json::Value>,
        ) -> Result<Self::Response, Self::DecodeError> {
            Ok(Response {
                data: None,
                errors: Some(
                    errors
                        .into_iter()
                        .map(serde_json::from_value)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                extensions: None,
            })
        }
    }

    /// A streaming operation for a GraphQLQuery
    pub struct StreamingOperation<Q: GraphQLQuery> {
        inner: QueryBody<Q::Variables>,
        phantom: PhantomData<Q>,
    }

    impl<Q: GraphQLQuery> StreamingOperation<Q> {
        /// Constructs a StreamingOperation
        pub fn new(variables: Q::Variables) -> Self {
            Self {
                inner: Q::build_query(variables),
                phantom: PhantomData,
            }
        }
    }

    impl<Q: GraphQLQuery> serde::Serialize for StreamingOperation<Q> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.inner.serialize(serializer)
        }
    }

    impl<Q: GraphQLQuery> GraphqlOperation for StreamingOperation<Q> {
        type Response = Response<Q::ResponseData>;

        type Error = serde_json::Error;

        fn decode(&self, response: serde_json::Value) -> Result<Self::Response, Self::Error> {
            serde_json::from_value(response)
        }
    }
}
