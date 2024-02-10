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
#[deprecated(
    since = "0.8.0-rc.1",
    note = "this trait is no longer needed with the new Client, please update to use that"
)]
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

#[cfg(feature = "client-cynic")]
mod cynic {
    use super::*;

    #[cfg_attr(docsrs, doc(cfg(feature = "client-cynic")))]
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
pub use self::graphql_client::StreamingOperation;

#[cfg(feature = "client-graphql-client")]
mod graphql_client {
    use super::*;
    use ::graphql_client::{GraphQLQuery, QueryBody, Response};
    use std::marker::PhantomData;

    /// A streaming operation for a GraphQLQuery
    #[cfg_attr(docsrs, doc(cfg(feature = "client-graphql-client")))]
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
