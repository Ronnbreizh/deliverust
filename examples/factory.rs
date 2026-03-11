use std::{sync::Arc, time::Duration};

use deliverust::*;
use tokio::{sync::broadcast, task::JoinHandle};

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
                publish(IronOre {}).await;
            }
        });
        self.handle = Some(handle);
    }
}

#[derive(Clone, Debug)]
struct IronIngot {}

struct Smelter {}
impl Subscriber<IronOre> for Smelter {
    fn handle(&self, _message: &IronOre) {
        tokio::spawn(publish(IronIngot {}));
    }
}

#[derive(Debug, Clone)]
struct IronPlate {}

struct Constructor {
    sender: broadcast::Sender<IronIngot>,
    handle: Option<JoinHandle<()>>,
}

impl Constructor {
    fn new() -> Self {
        let (sender, _) = broadcast::channel(2000);

        Self {
            sender,
            handle: None,
        }
    }
    fn start_plate_production(&mut self) {
        let mut receiver = self.sender.subscribe();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                let _ingot_1 = receiver.recv().await;
                let _ingot_2 = receiver.recv().await;
                let _ingot_3 = receiver.recv().await;

                interval.tick().await;
                let iron_plate = IronPlate {};
                println!("Produced iron plate: {iron_plate:?}!");
            }
        });
        self.handle = Some(handle);
    }
}

impl Subscriber<IronIngot> for Constructor {
    fn handle(&self, message: &IronIngot) {
        self.sender
            .send(message.clone())
            .expect("Failed to proceed ingot");
    }
}

#[tokio::main]
async fn main() {
    let mut mine = IronMine::default();

    let smelter = Arc::new(Smelter {});
    subscribe::<IronOre>(&smelter).await;

    let mut constructor = Constructor::new();
    constructor.start_plate_production();

    subscribe::<IronIngot>(&Arc::new(constructor)).await;
    // subscribe::<IronOre>(&constructor).await;
    // ^- not possible due to trait constraints

    mine.start_production();

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to await ctrl c");
}
