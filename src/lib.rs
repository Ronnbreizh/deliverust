use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use tokio::sync::RwLock as TokioRwLock;

type AnyCallback = Box<dyn Fn(&dyn Any) + Send + Sync>;

static REGISTRY: LazyLock<TokioRwLock<ModuleTable>> =
    LazyLock::new(|| TokioRwLock::new(ModuleTable::default()));

#[derive(Default)]
/// Core registry of all the modules.
///
pub struct ModuleTable {
    subscribers: HashMap<TypeId, Vec<AnyCallback>>,
}

impl ModuleTable {
    pub fn publish<Message: 'static + Any + Sized>(&self, message: Message) {
        let subs = self.subscribers.get(&message.type_id()).unwrap();
        for sub in subs {
            sub(&message)
        }
    }

    pub fn register<
        Message: 'static + Send + Sync + Any,
        Sub: 'static + Subscriber<Message> + Send + Sync,
    >(
        &mut self,
        subscriber: Arc<Sub>,
    ) {
        let type_id = TypeId::of::<Message>();

        let callback = Box::new(move |message: &dyn Any| {
            let message = message.downcast_ref::<Message>().unwrap(); // <- message is
            // WARNING: if the handle is blocking/taking long, then the publisher is drastically
            // slowed.
            subscriber.handle(message);
        });

        if let Some(subs) = self.subscribers.get_mut(&type_id) {
            subs.push(callback);
        } else {
            self.subscribers.insert(type_id, vec![callback]);
        }
    }
}

pub trait Publisher<T: 'static> {
    #[expect(async_fn_in_trait)]
    async fn publish(message: T) {
        REGISTRY.read().await.publish(message);
    }
}

pub trait SubscriberGen {
    #[expect(async_fn_in_trait)]
    async fn subscribe<T: 'static + Send + Sync + Any>(
        sub: Arc<impl 'static + Subscriber<T> + Send + Sync>,
    ) {
        sub.inner_subscribe().await;
    }
}

pub trait Subscriber<T: 'static + Send + Sync + Any> {
    #[expect(async_fn_in_trait)]
    async fn inner_subscribe(self: &Arc<Self>)
    where
        Self: 'static + Send + Sync + Sized,
    {
        REGISTRY.write().await.register(Arc::clone(self));
    }

    // WARNING: this method should be short and delegate async to an other work/task/whatever
    // otherwise this would block the publishing mecanismn, making other modules wait and loosing
    // the lovely benefit of async programming.
    // Also you can deadlock if you subscribe directly inside this function.
    fn handle(&self, _message: &T);
}
