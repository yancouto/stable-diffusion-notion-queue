use std::{str::FromStr, time::Duration};

use crate::types::{CommonArgs, Item, ItemOutput, Txt2Img};
use anyhow::{Context, Result};
use notion::{
    ids::DatabaseId,
    models::{
        paging::Paging,
        properties::{PropertyValue, SelectedValue},
        search::{
            DatabaseQuery, DatabaseSort, FilterCondition, PropertyCondition, SelectCondition,
            SortDirection,
        },
        Page,
    },
    NotionApi,
};

const TYPE: &str = "Type";
const TXT2IMG: &str = "txt2img";
const STATUS: &str = "Status";
const PRIORITY: &str = "Priority";
const ON_QUEUE: &str = "On queue";
const PROMPT: &str = "Prompt";
const STEPS: &str = "Steps";
const WIDTH: &str = "Width";
const HEIGHT: &str = "Height";

pub(crate) struct NotionIntegration {
    api: NotionApi,
    id: DatabaseId,
}

trait PageHelper {
    fn get(&self, name: &str) -> Result<&PropertyValue>;
    fn get_text(&self, name: &str) -> Result<&str>;
    fn get_select(&self, name: &str) -> Result<&str>;
    fn get_maybe_u64(&self, name: &str) -> Result<Option<u64>>;
}

impl PageHelper for Page {
    fn get(&self, name: &str) -> Result<&PropertyValue> {
        self.properties
            .properties
            .get(name)
            .with_context(|| format!("Missing {name}"))
    }

    fn get_text(&self, name: &str) -> Result<&str> {
        Ok(match self.get(name)? {
            PropertyValue::Text { rich_text, .. } => rich_text
                .iter()
                .next()
                .with_context(|| format!("Missing {name}"))?
                .plain_text(),
            other => anyhow::bail!("Unrecognized text when looking for {name}: {other:?}"),
        })
    }

    fn get_select(&self, name: &str) -> Result<&str> {
        Ok(match self.get(name)? {
            PropertyValue::Select {
                select: Some(SelectedValue { name, .. }),
                ..
            } => name.as_str(),
            other => anyhow::bail!("Unrecognized select when looking for {name}: {other:?}"),
        })
    }

    fn get_maybe_u64(&self, name: &str) -> Result<Option<u64>> {
        Ok(match self.get(name)? {
            PropertyValue::Number { number, .. } => number
                .as_ref()
                .map(|n| n.as_u64().context("Non u64 number"))
                .transpose()?,
            other => anyhow::bail!("Unrecognized number when looking for {name}: {other:?}"),
        })
    }
}

fn convert(page: Page) -> Result<Item> {
    Ok(match page.get_select(TYPE)? {
        TXT2IMG => Item::Txt2Img(Txt2Img {
            common_args: CommonArgs {
                prompt: page.get_text(PROMPT)?.to_string(),
                steps: page.get_maybe_u64(STEPS)?,
                w: page.get_maybe_u64(WIDTH)?,
                h: page.get_maybe_u64(HEIGHT)?,
            },
        }),
        other => anyhow::bail!("Unrecognized type: {other:?}"),
    })
}

impl NotionIntegration {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            api: NotionApi::new(std::env::var("NOTION_TOKEN").context("Missing NOTION_TOKEN")?)?,
            id: DatabaseId::from_str(
                &std::env::var("DATABASE_ID").context("Missing DATABASE_ID")?,
            )?,
        })
    }

    pub async fn get_item(&self) -> Result<Item> {
        let page = loop {
            let page = self
                .api
                .query_database(
                    &self.id,
                    DatabaseQuery {
                        // No sorting = order I put in the UI?
                        sorts: Some(vec![DatabaseSort {
                            property: Some(PRIORITY.to_string()),
                            timestamp: None,
                            direction: SortDirection::Descending,
                        }]),
                        filter: Some(FilterCondition {
                            property: STATUS.to_string(),
                            condition: PropertyCondition::Status(SelectCondition::Equals(
                                ON_QUEUE.to_string(),
                            )),
                        }),
                        paging: Some(Paging {
                            start_cursor: None,
                            page_size: Some(1),
                        }),
                    },
                )
                .await?
                .results
                .into_iter()
                .next();
            if let Some(page) = page {
                break page;
            } else {
                println!("No item in queue, sleeping 30s");
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        };
        println!("Page: {page:?}");
        match convert(page) {
            Ok(item) => Ok(item),
            Err(err) => {
                println!("Failed to convert page, marking it as error in Notion. {err}");
                todo!("implement changing page on notion crate");
                Err(err)
            }
        }
    }

    pub async fn save(&self, output: ItemOutput) -> Result<()> {
        todo!()
    }
}
