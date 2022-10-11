use crate::types::{CommonArgs, Item, ItemOutput};
use anyhow::Result;
use tokio::process::Command;

impl CommonArgs {
    fn into_args(self) -> Vec<String> {
        let mut args = vec!["--prompt".to_string(), self.prompt];
        if let Some(steps) = self.steps {
            // Careful. Might be different on txt2imghd when we add that
            args.push(format!("--ddim_steps={steps}"));
        }
        if let Some((w, h)) = self.w.zip(self.h) {
            args.extend([format!("--W={w}"), format!("--H={h}")]);
        }
        args
    }
}

pub(crate) struct StableDiffusionRunner;

impl StableDiffusionRunner {
    async fn run_impl(&self, item: Item) -> Result<()> {
        use Item::*;
        match item {
            Txt2Img(txt2img) => {
                let mut cmd = Command::new("conda");
                // TODO: Move to CLI arg
                let stable_diffusion_root = "/home/yancouto/Code/stable-diffusion";
                let txt2img_bin = "optimizedSD/optimized_txt2img.py";
                cmd.current_dir(stable_diffusion_root)
                    .args(["run", "--no-capture-output", "--name=ldm"])
                    .args(["python", txt2img_bin])
                    .args(txt2img.common_args.into_args())
                    .kill_on_drop(true);
                println!("Will run: {cmd:?}");
                anyhow::ensure!(cmd.status().await?.success(), "Process wasn't successful");
            }
        }
        Ok(())
    }

    pub(crate) async fn run(&self, item: Item) -> ItemOutput {
        match self.run_impl(item).await {
            Ok(_) => ItemOutput::Success,
            Err(_) => ItemOutput::Error,
        }
    }
}
