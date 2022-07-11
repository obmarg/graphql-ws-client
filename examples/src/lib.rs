#[cfg(not(target_arch = "wasm32"))]
mod tokio_spawner {
    pub struct TokioSpawner(tokio::runtime::Handle);

    impl TokioSpawner {
        pub fn new(handle: tokio::runtime::Handle) -> Self {
            TokioSpawner(handle)
        }

        pub fn current() -> Self {
            TokioSpawner::new(tokio::runtime::Handle::current())
        }
    }

    impl futures::task::Spawn for TokioSpawner {
        fn spawn_obj(
            &self,
            obj: futures::task::FutureObj<'static, ()>,
        ) -> Result<(), futures::task::SpawnError> {
            self.0.spawn(obj);
            Ok(())
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub use tokio_spawner::TokioSpawner;
