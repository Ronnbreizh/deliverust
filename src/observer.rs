use std::{
    any::TypeId,
    collections::HashMap,
    sync::atomic::{AtomicU64, AtomicUsize},
    time::{Duration, Instant},
};

/// `Observer` are used to monitor the `REGISTRY` health and the message usage.
pub trait ObserverBehavior: 'static + Send + Sync {
    /// What should be done on new message
    fn register_type(&mut self, type_id: TypeId);
    /// What should be done on new message
    fn increment_type(&self, type_id: TypeId);
    /// Return the number of messages of a given type
    fn count_message(&self, type_id: &TypeId) -> usize;

    /// Increase lock duration by duration in nanos
    fn increase_lock_duration(&self, duration: Duration);

    /// Monitor availability of the REGISTRY.
    /// The closer to 100%, the less efficient message broadcasting is.
    fn lock_duration(&self) -> u8;
}

/// The default observer, that should work for most people right out of the box.
#[derive(Debug)]
pub struct Observer {
    counter: HashMap<TypeId, AtomicUsize>,
    start_time: Instant,
    // lock duration in nano secs
    lock_duration: AtomicU64,
}

impl Default for Observer {
    fn default() -> Self {
        Self {
            counter: Default::default(),
            start_time: Instant::now(),
            lock_duration: AtomicU64::new(0),
        }
    }
}

impl ObserverBehavior for Observer {
    fn register_type(&mut self, type_id: TypeId) {
        self.counter.insert(type_id, AtomicUsize::new(0));
    }

    fn increment_type(&self, type_id: TypeId) {
        if let Some(value) = self.counter.get(&type_id) {
            value.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn count_message(&self, type_id: &TypeId) -> usize {
        self.counter
            .get(type_id)
            .map(|atomic| atomic.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or_default()
    }

    fn lock_duration(&self) -> u8 {
        let life_duration = Instant::now().duration_since(self.start_time);

        let ratio = self
            .lock_duration
            .load(std::sync::atomic::Ordering::Relaxed)
            * 100
            / life_duration.as_nanos() as u64;

        ratio.clamp(0, 100) as u8
    }

    fn increase_lock_duration(&self, duration: Duration) {
        self.lock_duration.fetch_add(
            duration.as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{any::TypeId, sync::Arc, time::Duration};

    use crate::{
        Subscriber, evaluate,
        observer::{Observer, ObserverBehavior},
        publish, register_observer, subscribe,
    };

    #[derive(Debug)]
    struct SlowHandler {}
    #[derive(Debug)]
    struct FastHandler {}
    #[derive(Debug)]
    struct Message {}

    impl Subscriber<Message> for SlowHandler {
        fn handle(&self, _message: &Message) {
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    impl Subscriber<Message> for FastHandler {
        fn handle(&self, _message: &Message) {}
    }

    #[test]
    fn load_rate() {
        let observer = Observer::default();
        assert_eq!(observer.lock_duration(), 0);
    }

    #[test]
    fn increase_load_rate() {
        let observer = Observer::default();
        observer.increase_lock_duration(Duration::from_secs(1));
        assert!(observer.lock_duration() > 99);
    }

    #[tokio::test]
    async fn count_message() {
        let (channel_tx, mut channel_rx) = tokio::sync::mpsc::channel(1);
        let fast_handler = Arc::new(FastHandler {});
        subscribe(&fast_handler).await;
        publish(Message {}).await;

        evaluate(async move |obs| {
            let load_rate = obs.count_message(&TypeId::of::<Message>());
            channel_tx.send(load_rate).await.unwrap();
        })
        .await;

        let message_count = channel_rx.recv().await.unwrap();
        assert_eq!(message_count, 1);
        publish(Message {}).await;
    }

    #[tokio::test]
    async fn long_lock() {
        let (channel_tx, mut channel_rx) = tokio::sync::mpsc::channel(1);

        let slow_handler = Arc::new(SlowHandler {});
        subscribe(&slow_handler).await;

        register_observer(Observer::default()).await;
        publish(Message {}).await;

        evaluate(async move |obs| {
            let load_rate = obs.lock_duration();
            channel_tx.send(load_rate).await.unwrap();
        })
        .await;

        let load_rate = channel_rx.recv().await.unwrap();
        assert_eq!(load_rate, 99);
    }
}
