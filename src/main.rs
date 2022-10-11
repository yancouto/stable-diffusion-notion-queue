use std::time::Duration;

mod notion_integration;
mod stable_difussion_runner;
mod types;

use notion_integration::NotionIntegration;
use stable_difussion_runner::StableDiffusionRunner;

#[tokio::main]
async fn main() {
    let notion = NotionIntegration::from_env().expect("Couldn't create Notion API");
    let runner = StableDiffusionRunner;
    loop {
        let item = match notion.get_item().await {
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
        if let Err(err) = notion.save(output).await {
            println!("Error saving item output: {err}");
        }
    }
}
