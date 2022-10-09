use std::str::FromStr;

use crate::types::Item;
use anyhow::{Context, Result};
use notion::{
    ids::DatabaseId,
    models::{
        paging::Paging,
        search::{
            DatabaseQuery, DatabaseSort, FilterCondition, PropertyCondition, SelectCondition,
            SortDirection,
        },
    },
    NotionApi,
};

pub(crate) struct NotionQueueReader {
    api: NotionApi,
    id: DatabaseId,
}

impl NotionQueueReader {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            api: NotionApi::new(std::env::var("NOTION_TOKEN").context("Missing NOTION_TOKEN")?)?,
            id: DatabaseId::from_str(
                &std::env::var("DATABASE_ID").context("Missing DATABASE_ID")?,
            )?,
        })
    }

    pub async fn get_item(&self) -> Result<Item> {
        let page = self
            .api
            .query_database(
                &self.id,
                DatabaseQuery {
                    // No sorting = order I put in the UI?
                    sorts: Some(vec![DatabaseSort {
                        property: Some("Priority".to_string()),
                        timestamp: None,
                        direction: SortDirection::Descending,
                    }]),
                    filter: Some(FilterCondition {
                        property: "Status".to_string(),
                        condition: PropertyCondition::Status(SelectCondition::Equals(
                            "On queue".to_string(),
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
        println!("Page: {:?}", page);
        todo!()
    }
}
