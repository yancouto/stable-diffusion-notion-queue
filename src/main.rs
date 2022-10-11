use std::time::Duration;

mod notion_queue_reader;
mod types;

use notion_queue_reader::NotionQueueReader;
use types::{Item, ItemOutput};

impl Item {
    async fn run(self) -> ItemOutput {
        todo!()
    }
}

impl ItemOutput {
    async fn save(self) {
        todo!()
    }
}

#[tokio::main]
async fn main() {
    let item_getter = NotionQueueReader::from_env().expect("Couldn't create Notion API");
    loop {
        let item = match item_getter.get_item().await {
            Ok(item) => item,
            Err(err) => {
                println!("Error getting item: {err}");
                println!("Sleeping 60s");
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }
        };
        println!("Item: {item:?}");
        let output = item.run().await;
        output.save().await;
    }
}
