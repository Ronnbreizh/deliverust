use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, Weak},
    time::Instant,
};

use tokio::sync::mpsc as tokio_mpsc;
use tokio::task::JoinHandle as TokioHandle;

use crate::Subscriber;

#[derive(Default)]
pub struct AsyncModuleTable {
    /// Tokio tasks for given type of message
    workers: HashMap<TypeId, MessageWorker>,
}

enum ControlWorker {
    Publish(Box<dyn Any + Send + Sync>),
    Subscriber(Weak<dyn Any + Send + Sync>),
}

/// Used to dispatch dynamically messages
pub struct MessageWorker {
    /// TypeId which this worker enforce
    type_id: TypeId,
    /// Channel entry
    tx: tokio_mpsc::Sender<ControlWorker>,
    /// Underlying task
    task: TokioHandle<()>,
}

impl MessageWorker {
    async fn add_subscriber(&self, sub: Weak<dyn Any + Send + Sync>) {
        self.tx.send(ControlWorker::Subscriber(sub)).await.unwrap();
    }
}

impl AsyncModuleTable {
    pub async fn publish<Message: Any + Send + Sync>(&self, message: Message) {
        if let Some(worker) = self.workers.get(&TypeId::of::<Message>()) {
            // TODO: handle this unwrap
            worker
                .tx
                .send(ControlWorker::Publish(Box::new(message)))
                .await
                .unwrap();
        }
    }
    pub async fn add_subcriber<Message: 'static + Send + Sync + Any>(
        &self,
        sub: &Arc<impl Subscriber<Message> + Send + Sync + Any + 'static>,
    ) {
        if let Some(worker) = self.workers.get(&TypeId::of::<Message>()) {
            let weak = Arc::downgrade(sub);
            // TODO: handle this unwrap
            worker.add_subscriber(weak).await;
        }
    }

    pub fn spawn_message_worker<
        Message: 'static + Send + Sync + Any,
        Sub: 'static + Subscriber<Message> + Send + Sync,
    >() -> MessageWorker {
        //let (tx, mut rx) = tokio_mpsc::channel::<Box<dyn Any + Send + Sync>>(100);
        let (tx, mut rx) = tokio_mpsc::channel::<ControlWorker>(100);
        let mut subs: Vec<Weak<dyn Any + Send + Sync>> = Vec::new();
        let task = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                match message {
                    ControlWorker::Publish(message) => {
                        let Some(message) = message.downcast_ref::<Message>() else {
                            print!("Not a valid message mon reuf");
                            continue;
                        };
                        for sub in subs.iter() {
                            if let Some(strong) = sub.upgrade() {
                                let Some(sub) = strong.downcast_ref::<Sub>() else {
                                    continue;
                                };
                                sub.async_handle(message).await;
                            } else {
                                println!("Should drop this sub from the vec")
                            }
                        }
                    }
                    ControlWorker::Subscriber(weak) => {
                        subs.push(weak);
                    }
                }
            }
            println!("JOB DONE");
        });

        MessageWorker {
            type_id: TypeId::of::<Message>(),
            tx,
            task,
        }
    }

    /// Add an async subscriber
    /// What if this item is registered twice ?
    pub async fn register_async<
        Message: 'static + Send + Sync + Any,
        Sub: 'static + Subscriber<Message> + Send + Sync,
    >(
        &mut self,
        subscriber: Arc<Sub>,
    ) {
        let _begin = Instant::now();
        let weak_sub = Arc::downgrade(&subscriber);
        let type_id = TypeId::of::<Message>();

        let worker = self
            .workers
            .entry(type_id)
            .or_insert(Self::spawn_message_worker::<Message, Sub>());
        worker.add_subscriber(weak_sub).await;
    }
}
