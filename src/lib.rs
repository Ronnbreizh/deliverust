use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
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
    pub fn publish<Message: 'static + Any + Sized + Debug>(&self, message: Message) {
        if let Some(subs) = self.subscribers.get(&message.type_id()) {
            for sub in subs {
                sub(&message)
            }
        } else {
            println!("No sub for {message:?}");
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

pub async fn subscribe<T: Send + Sync + Any>(
    inner: &Arc<impl Subscriber<T> + Send + Sync + 'static>,
) {
    REGISTRY.write().await.register(Arc::clone(inner));
}

pub async fn publish<T: 'static + Send + Sync + Debug>(message: T) {
    REGISTRY.read().await.publish(message);
}

pub trait Subscriber<T: 'static + Send + Sync + Any> {
    // WARNING: this method should be short and delegate async to an other work/task/whatever
    // otherwise this would block the publishing mecanismn, making other modules wait and loosing
    // the lovely benefit of async programming.
    // Also you can deadlock if you subscribe directly inside this function.
    fn handle(&self, _message: &T);
}
