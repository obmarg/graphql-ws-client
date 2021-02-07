/// An abstraction over GraphQL operations.
///
/// This allows the library to be used with different underlying GraphQL
/// clients.
pub trait GraphqlOperation: serde::Serialize {
    /// The "generic" response type.  A graphql-ws-client supports running multiple
    /// operations at a time.  This GenericResponse is what will intially be decoded -
    /// with the `decode` function converting this into the actual operation response.
    ///
    /// This type needs to match up with `GraphqlClient::Response` below.
    type GenericResponse;

    /// The actual response & error type of this operation.
    type Response;
    type Error: std::error::Error;

    /// Decodes a GenericResponse into the actual response for this operation.
    fn decode(&self, data: Self::GenericResponse) -> Result<Self::Response, Self::Error>;
}

/// A trait for GraphQL clients.
///
/// This allows the library to be used with different underlying GraphQL
/// clients.
///
/// Adding an impl of this and `GraphqlOperation` for a new client should allow graphql-ws-
/// client to be used with operations from that client.
pub trait GraphqlClient {
    type Response: serde::de::DeserializeOwned + Send;

    type DecodeError: std::error::Error + Send + 'static;

    type Operation: Send;

    fn error_response(errors: Vec<serde_json::Value>) -> Result<Self::Response, Self::DecodeError>;
}
