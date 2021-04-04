//! This module contains traits that abstract over GraphQL implementations,
//! allowing this library to be used with different GraphQL client libraries.
//!
//! Support is provided for [`cynic`][cynic], but other client libraries can
//! be added by implementing these traits for those libraries.
//!
//! [cynic]: https://cynic-rs.dev

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
    /// The "generic" response type.  A graphql-ws-client supports running multiple
    /// operations at a time.  This GenericResponse is what will intially be decoded -
    /// with the `decode` function converting this into the actual operation response.
    ///
    /// This type needs to match up with [GraphqlClient::Response].
    type GenericResponse;

    /// The actual response & error type of this operation.
    type Response;

    /// The error that will be returned from failed attempts to decode a `Response`.
    type Error: std::error::Error;

    /// Decodes a `GenericResponse` into the actual response that will be returned
    /// to users for this operation.
    fn decode(&self, data: Self::GenericResponse) -> Result<Self::Response, Self::Error>;
}


#[cfg(feature = "cynic")]
pub use self::cynic::Cynic;

#[cfg(feature = "cynic")]
mod cynic {
    use super::*;

    /// Provides an implementation of [GraphqlClient] for the cynic GraphQL crate
    pub struct Cynic {}

    impl GraphqlClient for Cynic {
        type Response = ::cynic::GraphQLResponse<serde_json::Value>;

        type DecodeError = serde_json::Error;

        fn error_response(
            errors: Vec<serde_json::Value>,
        ) -> Result<Self::Response, Self::DecodeError> {
            Ok(::cynic::GraphQLResponse {
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

    impl<'a, ResponseData> GraphqlOperation for ::cynic::StreamingOperation<'a, ResponseData>
    where
        ResponseData: 'a,
    {
        type GenericResponse = ::cynic::GraphQLResponse<serde_json::Value>;

        type Response = ::cynic::GraphQLResponse<ResponseData>;

        type Error = ::cynic::DecodeError;

        fn decode(&self, data: Self::GenericResponse) -> Result<Self::Response, Self::Error> {
            self.decode_response(data)
        }
    }
}
