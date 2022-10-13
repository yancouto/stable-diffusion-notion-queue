use notion::ids::PageId;

#[derive(Debug)]
pub(crate) struct Item {
    pub page_id: PageId,
    pub cmd: SdCommand,
}

#[derive(Debug)]
pub(crate) enum SdCommand {
    Txt2Img(Txt2Img),
}

#[derive(Debug)]
pub(crate) struct CommonArgs {
    pub prompt: String,
    pub steps: Option<u64>,
    pub w: Option<u64>,
    pub h: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct Txt2Img {
    pub common_args: CommonArgs,
}

#[must_use]
pub(crate) struct ItemOutput {
    pub page_id: PageId,
    pub result: anyhow::Result<()>,
}
