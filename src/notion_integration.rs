use std::{collections::HashMap, str::FromStr, time::Duration};

use crate::types::{CommonArgs, Item, ItemOutput, SdCommand, Txt2Img};
use anyhow::{Context, Result};
use notion::{
    ids::{DatabaseId, PageId},
    models::{
        page::UpdatePageQuery,
        paging::Paging,
        properties::{PropertyValue, SelectedValue, WritePropertyValue, WriteSelectedValue},
        search::{
            DatabaseQuery, DatabaseSort, FilterCondition, PropertyCondition, SelectCondition,
            SortDirection,
        },
        text::{RichText, RichTextCommon, Text},
        Page, WriteProperties,
    },
    NotionApi,
};

const TYPE: &str = "Type";
const TXT2IMG: &str = "txt2img";
const STATUS: &str = "Status";
const STATUS_FAILED: &str = "Failed";
const STATUS_DONE: &str = "Done";
const STATUS_IN_PROGRESS: &str = "In progress";
const PRIORITY: &str = "Priority";
const ON_QUEUE: &str = "On queue";
const PROMPT: &str = "Prompt";
const STEPS: &str = "Steps";
const WIDTH: &str = "Width";
const HEIGHT: &str = "Height";
const ERROR: &str = "Error";

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
    let cmd = match page.get_select(TYPE)? {
        TXT2IMG => SdCommand::Txt2Img(Txt2Img {
            common_args: CommonArgs {
                prompt: page.get_text(PROMPT)?.to_string(),
                steps: page.get_maybe_u64(STEPS)?,
                w: page.get_maybe_u64(WIDTH)?,
                h: page.get_maybe_u64(HEIGHT)?,
            },
        }),
        other => anyhow::bail!("Unrecognized type: {other:?}"),
    };
    Ok(Item {
        page_id: page.id,
        cmd,
    })
}

enum Status {
    // TODO: Add results, like the image links
    Ok,
    InProgress,
    Err { reason: String },
}

fn select(name: String) -> WriteSelectedValue {
    WriteSelectedValue {
        id: None,
        name: Some(name),
        color: None,
    }
}

fn text(txt: String) -> WritePropertyValue {
    WritePropertyValue::Text {
        rich_text: vec![RichText::Text {
            rich_text: RichTextCommon {
                plain_text: txt.clone(),
                href: None,
                annotations: None,
            },
            text: Text {
                content: txt,
                link: None,
            },
        }],
    }
}

impl Status {
    fn status_str(&self) -> String {
        match self {
            Self::Err { .. } => STATUS_FAILED.to_string(),
            Self::InProgress => STATUS_IN_PROGRESS.to_string(),
            Self::Ok => STATUS_DONE.to_string(),
        }
    }

    fn add_extra(self, properties: &mut HashMap<String, WritePropertyValue>) {
        match self {
            Self::Err { reason } => {
                properties.insert(ERROR.to_string(), text(reason));
            }
            Self::Ok => {}
            Self::InProgress => {}
        }
    }
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

    async fn update_status(&self, page_id: PageId, status: Status) -> Result<()> {
        let mut properties = HashMap::new();
        properties.insert(
            STATUS.to_string(),
            WritePropertyValue::Status {
                status: Some(select(status.status_str())),
            },
        );
        status.add_extra(&mut properties);
        self.api
            .update_page(
                page_id,
                UpdatePageQuery {
                    properties: Some(WriteProperties { properties }),
                    ..Default::default()
                },
            )
            .await?;
        Ok(())
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
        let page_id = page.id.clone();
        match convert(page) {
            Ok(item) => {
                self.update_status(page_id, Status::InProgress).await?;
                Ok(item)
            }
            Err(err) => {
                println!("Failed to convert page, marking it as error in Notion. {err}");
                self.update_status(
                    page_id,
                    Status::Err {
                        reason: format!("Couldn't convert page: {err}"),
                    },
                )
                .await?;
                Err(err)
            }
        }
    }

    pub async fn save(&self, output: ItemOutput) -> Result<()> {
        match output.result {
            Ok(()) => self.update_status(output.page_id, Status::Ok).await?,
            Err(err) => {
                self.update_status(
                    output.page_id,
                    Status::Err {
                        reason: format!("Failed to run cmd: {err}"),
                    },
                )
                .await?
            }
        }
        Ok(())
    }
}
