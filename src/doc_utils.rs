use crate::{next::Message, Error};

pub struct Conn;

#[async_trait::async_trait]
impl crate::next::Connection for Conn {
    async fn receive(&mut self) -> Option<Message> {
        unimplemented!()
    }

    async fn send(&mut self, _: Message) -> Result<(), Error> {
        unimplemented!()
    }
}

#[derive(serde::Serialize)]
pub struct Subscription;

impl crate::graphql::GraphqlOperation for Subscription {
    type Response = ();

    type Error = crate::Error;

    fn decode(&self, _data: serde_json::Value) -> Result<Self::Response, Self::Error> {
        unimplemented!()
    }
}
