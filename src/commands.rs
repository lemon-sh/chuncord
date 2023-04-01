use std::{
    borrow::Cow,
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    io::{self, Seek, SeekFrom},
    path::Path,
};

use color_eyre::{
    eyre::{eyre, ContextCompat},
    Result,
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    discord::{self, DiscordMessage},
    read_ext::ReadExt,
};

const MAXBUF: u64 = 8_380_416;

fn get_webhook(webhook: Option<&str>) -> Result<String> {
    if let Some(webhook) = webhook {
        if let Some(webhook) = webhook.strip_prefix("raw:") {
            return Ok(webhook.into());
        }
        return Config::load()?
            .webhooks
            .remove(webhook)
            .ok_or_else(|| eyre!("Webhook named '{webhook}' not found."));
    }
    let mut cfg = Config::load()?;
    let default_webhook = cfg
        .default_webhook
        .ok_or_else(|| eyre!("No default webhook set"))?;
    cfg.webhooks
        .remove(&default_webhook)
        .ok_or_else(|| eyre!("Default webhook '{default_webhook}' not found."))
}

fn progress_bars() -> (ProgressStyle, ProgressStyle) {
    let style_int = ProgressStyle::with_template(
        "{spinner:.green} [{bar:40.blue}] {pos}/{len} | {wide_msg:.green}",
    )
    .unwrap()
    .progress_chars("-> ");

    let style_data = ProgressStyle::with_template(
		"{spinner:.green} [{bar:40.blue}] {bytes}/{total_bytes} {bytes_per_sec} | {wide_msg:.green}",
	)
	.unwrap()
	.progress_chars("-> ");

    (style_int, style_data)
}

#[derive(Serialize, Deserialize)]
struct Index<'a> {
    filename: Cow<'a, str>,
    filesize: u64,
    parts: HashMap<u64, String>,
}

pub fn upload(file: &str, webhook: Option<&str>) -> Result<()> {
    let webhook = get_webhook(webhook)?;
    let filename = Path::new(file)
        .file_name()
        .and_then(OsStr::to_str)
        .wrap_err("The path is invalid")?;
    let mut file = File::open(file)?;

    let filesize = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(0))?;

    let mut parts_num = filesize / MAXBUF;
    if filesize % MAXBUF > 0 {
        parts_num += 1;
    }

    let mut buffer = vec![0u8; MAXBUF as usize];
    let mut parts = HashMap::with_capacity(parts_num as usize);

    let mpb = MultiProgress::new();
    let pb_file = mpb.add(ProgressBar::new(filesize));
    let pb_part = mpb.add(ProgressBar::new(parts_num));

    let (style_int, style_data) = progress_bars();

    pb_file.set_style(style_data);
    pb_part.set_style(style_int);

    let mut file = pb_file.wrap_read(file);
    for part in pb_part.wrap_iter(0..parts_num) {
        let part_size = file.read_max(&mut buffer)?;
        let response =
            discord::upload(format!("chuncord_{part}"), &buffer[0..part_size], &webhook)?;
        parts.insert(response.id, response.attachment.url);
    }

    let index = Index {
        filename: filename.into(),
        filesize,
        parts,
    };
    let index_json = serde_json::to_string_pretty(&index)?;
    let response = discord::upload("chuncord_index".into(), index_json.as_bytes(), &webhook)?;
    println!(
        "Done!\nURL: {}\nMID (required for delete): {}",
        response.id, response.attachment.url
    );
    Ok(())
}

pub fn download(index_url: &str, filename: Option<&str>) -> Result<()> {
    println!("Downloading index...");
    let index: Index = serde_json::from_reader(ureq::get(index_url).call()?.into_reader())?;
    let file = filename.unwrap_or(&index.filename);

    let mpb = MultiProgress::new();
    let pb_file = mpb.add(ProgressBar::new(index.filesize));
    let pb_part = mpb.add(ProgressBar::new(index.parts.len() as u64));

    let (style_int, style_data) = progress_bars();
    pb_file.set_style(style_data);
    pb_part.set_style(style_int);

    let mut file = pb_file.wrap_write(File::create(file)?);

    for part in pb_part.wrap_iter(index.parts.into_values()) {
        let mut reader = ureq::get(&part).call()?.into_reader();
        io::copy(&mut reader, &mut file)?;
    }

    println!("\nDone!");
    Ok(())
}

pub fn delete(mid: u64, webhook: Option<&str>) -> Result<()> {
    let webhook = get_webhook(webhook)?;
    println!("Downloading index...");
    let index_message_json = ureq::get(&format!("{webhook}/messages/{mid}"))
        .call()?
        .into_reader();
    let index_message: DiscordMessage = serde_json::from_reader(index_message_json)?;
    let index_url = index_message.attachment.url;
    let index_json = ureq::get(&index_url).call()?.into_reader();
    let index: Index = serde_json::from_reader(index_json)?;

    let pb = ProgressBar::new(index.parts.len() as u64);
    pb.set_style(progress_bars().0);

    for part in pb.wrap_iter(index.parts.into_keys()) {
        discord::delete(part, &webhook)?;
    }

    println!("\nDeleting index...");
    discord::delete(mid, &webhook)?;
    println!("\nDone!");
    Ok(())
}
