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
