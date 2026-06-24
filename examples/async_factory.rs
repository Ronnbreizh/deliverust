use std::{sync::Arc, time::Duration};

use deliverust::deliverust_async_trait;
use deliverust::*;
use tokio::{sync::Mutex as TokioMutex, task::JoinHandle};
#[derive(Clone, Debug)]
struct IronOre {}

#[derive(Default)]
struct IronMine {
    handle: Option<JoinHandle<()>>,
}

impl IronMine {
    fn start_production(&mut self) {
        let handle = tokio::spawn(async {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                async_publish(IronOre {}).await;
            }
        });
        self.handle = Some(handle);
    }
}

#[derive(Clone, Debug)]
struct IronIngot {}

struct Smelter {}

#[deliverust_async_trait]
impl Subscriber<IronOre> for Smelter {
    async fn async_handle(&self, _message: &IronOre) {
        async_publish(IronIngot {}).await
    }

    fn handle(&self, _message: &IronOre) {
        todo!("Will not be used")
    }
}

#[derive(Debug, Clone)]
struct IronPlate {}

#[derive(Default)]
struct Constructor {
    counter: TokioMutex<usize>,
}

#[derive(Default)]
struct Storage {
    counter: TokioMutex<usize>,
}

#[deliverust_async_trait]
impl Subscriber<IronPlate> for Storage {
    fn handle(&self, _message: &IronPlate) {
        todo!()
    }

    async fn async_handle(&self, _message: &IronPlate) {
        let mut value = self.counter.lock().await;
        *value += 1;
        println!("Storage contains {value} plates");
    }
}

#[deliverust_async_trait]
impl Subscriber<IronIngot> for Constructor {
    fn handle(&self, _message: &IronIngot) {
        todo!("Wont be used")
    }

    async fn async_handle(&self, _message: &IronIngot) {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        if *counter >= 3 {
            *counter -= 3;
            drop(counter);
            tokio::time::sleep(Duration::from_secs(3)).await;
            async_publish(IronPlate {}).await;
        }
    }
}

#[tokio::main]
async fn main() {
    let mut mine = IronMine::default();

    let smelter = Arc::new(Smelter {});
    async_subscribe::<IronOre>(&smelter).await;

    let constructor = Arc::new(Constructor::default());
    async_subscribe::<IronIngot>(&constructor).await;
    let storage = Arc::new(Storage::default());
    async_subscribe::<IronPlate>(&storage).await;

    mine.start_production();

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to await ctrl c");
}
