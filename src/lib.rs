use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, LazyLock},
    time::Instant,
};

use tokio::sync::RwLock as TokioRwLock;

use crate::{async_module_table::AsyncModuleTable, observer::ObserverBehavior};

/// We expose async_trait for conveniences for users of the crate.
pub use async_trait::async_trait as deliverust_async_trait;
pub mod async_module_table;
pub mod observer;
pub mod subscriber;
pub use subscriber::Subscriber;

type AnyCallback = Box<dyn Fn(&dyn Any) + Send + Sync>;

static REGISTRY: LazyLock<TokioRwLock<ModuleTable>> =
    LazyLock::new(|| TokioRwLock::new(ModuleTable::default()));

static ASYNC_REGISTRY: LazyLock<TokioRwLock<AsyncModuleTable>> =
    LazyLock::new(|| TokioRwLock::new(AsyncModuleTable::default()));

#[derive(Default)]
/// Core registry of all the modules.
///
pub struct ModuleTable {
    subscribers: HashMap<TypeId, Vec<AnyCallback>>,
    observer: Option<Box<dyn ObserverBehavior>>,
}

impl ModuleTable {
    fn observer_wrapper(&self, cb: impl Fn(&Self)) {
        let begin = Instant::now();

        cb(self);

        if let Some(observer) = &self.observer {
            let duration = Instant::now().duration_since(begin);
            observer.increase_lock_duration(duration);
        }
    }

    pub fn publish<Message: 'static + Any + Sized + Debug>(&self, message: Message) {
        self.observer_wrapper(|me| {
            if let Some(subs) = me.subscribers.get(&message.type_id()) {
                for sub in subs {
                    sub(&message)
                }
            }
        });
        if let Some(observer) = &self.observer {
            observer.increment_type(TypeId::of::<Message>());
        }
    }

    /// Add a subscriber
    pub fn register<
        Message: 'static + Send + Sync + Any,
        Sub: 'static + Subscriber<Message> + Send + Sync,
    >(
        &mut self,
        subscriber: Arc<Sub>,
    ) {
        let begin = Instant::now();
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
            // New kind of subscriber, yay
            self.subscribers.insert(type_id, vec![callback]);
        }

        if let Some(observer) = &mut self.observer {
            observer.register_type(type_id);
        }

        if let Some(observer) = &self.observer {
            let duration = Instant::now().duration_since(begin);
            observer.increase_lock_duration(duration);
        }
    }
}

pub(crate) async fn subscribe_inner<T: Send + Sync + Any>(
    registry: &TokioRwLock<ModuleTable>,
    inner: &Arc<impl Subscriber<T> + Send + Sync + 'static>,
) {
    registry.write().await.register(Arc::clone(inner));
}

/// Register the given type to a kind of messages.
pub async fn subscribe<T: Send + Sync + Any>(
    inner: &Arc<impl Subscriber<T> + Send + Sync + 'static>,
) {
    subscribe_inner(&REGISTRY, inner).await;
}

pub(crate) async fn async_subscribe_inner<T: Send + Sync + Any>(
    registry: &TokioRwLock<AsyncModuleTable>,
    inner: &Arc<impl Subscriber<T> + Send + Sync + Any + 'static>,
) {
    registry.write().await.add_subcriber(inner).await;
}

/// Register the given type to a kind of messages.
pub async fn async_subscribe<T: Send + Sync + Any>(
    inner: &Arc<impl Subscriber<T> + Send + Sync + 'static>,
) {
    async_subscribe_inner(&ASYNC_REGISTRY, inner).await;
}

pub async fn publish<T: 'static + Send + Sync + Debug>(message: T) {
    publish_inner(&REGISTRY, message).await;
}
pub(crate) async fn publish_inner<T: 'static + Send + Sync + Debug>(
    registry: &TokioRwLock<ModuleTable>,
    message: T,
) {
    registry.read().await.publish(message);
}

pub async fn async_publish<T: 'static + Send + Sync + Debug>(message: T) {
    async_publish_inner(&ASYNC_REGISTRY, message).await;
}
pub(crate) async fn async_publish_inner<T: 'static + Send + Sync + Debug>(
    registry: &TokioRwLock<AsyncModuleTable>,
    message: T,
) {
    registry.read().await.publish(message).await;
}

/// Register an `ObserverBehavior` to follow the ModuleTable load
pub async fn register_observer(observer: impl ObserverBehavior) {
    register_observer_inner(&REGISTRY, observer).await;
}

pub(crate) async fn register_observer_inner(
    registry: &TokioRwLock<ModuleTable>,
    observer: impl ObserverBehavior,
) {
    registry.write().await.observer = Some(Box::new(observer));
}

pub async fn evaluate(cb: impl AsyncFn(&Box<dyn ObserverBehavior>)) {
    evaluate_inner(&REGISTRY, cb).await;
}

pub async fn evaluate_inner(
    registry: &TokioRwLock<ModuleTable>,
    cb: impl AsyncFn(&Box<dyn ObserverBehavior>),
) {
    if let Some(observer) = &registry.read().await.observer {
        cb(observer).await;
    }
}

#[cfg(test)]
pub(crate) async fn create_registry_with_observer() -> TokioRwLock<ModuleTable> {
    let registry = TokioRwLock::new(ModuleTable::default());
    register_observer_inner(&registry, observer::Observer::default()).await;
    registry
}
