use anyhow::Result;
use std::time::Duration;

#[must_use]
struct Item;

#[must_use]
enum ItemOutput {
    Success,
    Error,
}

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

async fn get_item() -> Result<Item> {
    todo!()
}

#[tokio::main]
async fn main() {
    loop {
        let item = match get_item().await {
            Ok(item) => item,
            Err(err) => {
                println!("Error getting item: {err}");
                println!("Sleeping 60s");
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }
        };
        let output = item.run().await;
        output.save().await;
    }
}
