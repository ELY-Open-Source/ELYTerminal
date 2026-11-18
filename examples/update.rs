//! Installs release TAG over the copy at TARGET as Settings' Update does: `cargo run --example update -- TAG TARGET`.

use std::path::PathBuf;

use anyhow::Context as _;
use elyterminal::updates;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("elyterminal=info"))
        .init();
    let mut args = std::env::args().skip(1);
    let tag = args
        .next()
        .context("update: a release's tag, such as v0.2.5")?;
    let target = PathBuf::from(args.next().context("update: the copy to update")?);
    let release = updates::release(Some(&tag))?;
    let folder = target
        .parent()
        .context("update: the copy lies in no folder")?
        .join("update-download");
    let file = updates::install::fetch(&release.assets, tag.trim_start_matches('v'), &folder)?;
    updates::install::put(&file, &target)?;
    log::info!("update: {} holds {tag}", target.display());
    Ok(())
}
