mod client;
mod protocol;

pub mod graphql;
pub mod websockets;

pub use client::AsyncWebsocketClient;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
