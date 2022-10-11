use std::time::Duration;

mod notion_queue_reader;
mod stable_difussion_runner;
mod types;

use notion_queue_reader::NotionQueueReader;
use stable_difussion_runner::StableDiffusionRunner;
use types::ItemOutput;

impl ItemOutput {
    async fn save(self) {
        todo!()
    }
}

#[tokio::main]
async fn main() {
    let item_getter = NotionQueueReader::from_env().expect("Couldn't create Notion API");
    let runner = StableDiffusionRunner;
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
        let output = runner.run(item).await;
        output.save().await;
    }
}
