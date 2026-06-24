use std::any::Any;

use async_trait::async_trait;

#[async_trait]
pub trait Subscriber<T: 'static + Send + Sync + Any> {
    // WARNING: this method should be short and delegate async to an other work/task/whatever
    // otherwise this would block the publishing mecanismn, making other modules wait and loosing
    // the lovely benefit of async programming.
    // Also you can deadlock if you subscribe directly inside this function.
    fn handle(&self, _message: &T);
    async fn async_handle(&self, _message: &T) {}
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use crate::Subscriber;

    struct DummyStub {}

    struct Message {}

    #[async_trait]
    impl Subscriber<Message> for DummyStub {
        fn handle(&self, _message: &Message) {
            println!("Nie");
        }
        async fn async_handle(&self, _message: &Message) {}
    }
}
