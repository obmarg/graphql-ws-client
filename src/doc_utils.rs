use futures::{future::BoxFuture, Future};

use crate::{next::Message, Error};

pub struct Conn;

impl crate::next::Connection for Conn {
    fn receive(&mut self) -> BoxFuture<'_, Option<Message>> {
        unimplemented!()
    }

    fn send(&mut self, _: Message) -> BoxFuture<'_, Result<(), Error>> {
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

pub fn spawn<T>(_future: impl Future<Output = T> + Send + 'static) {}
