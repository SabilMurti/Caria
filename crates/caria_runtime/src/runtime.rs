use std::future::Future;
use tokio::runtime::Runtime;

pub struct CariaRuntime {
    rt: Runtime,
}

impl CariaRuntime {
    pub fn new() -> Self {
        Self {
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
        }
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.rt.block_on(future)
    }

    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.rt.spawn(future);
    }
}
