use std::{
    borrow::Cow,
    collections::hash_map::Entry,
    ffi::OsStr,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
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
};

const MAXBUF: u64 = 26_210_000;

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
    let style_int =
        ProgressStyle::with_template("{spinner:.green} [{bar:40.blue}] part {pos}/{len}")
            .unwrap()
            .progress_chars("=> ");

    let style_data =
        ProgressStyle::with_template("{spinner:.green} [{bar:40.blue}] {bytes}/{total_bytes}")
            .unwrap()
            .progress_chars("=> ");

    (style_int, style_data)
}

#[derive(Serialize, Deserialize)]
struct Index<'a> {
    name: Cow<'a, str>,
    size: u64,
    parts: Vec<(u64, Cow<'a, str>)>,
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

    let mut parts = Vec::with_capacity(usize::try_from(parts_num)?);

    let mpb = MultiProgress::new();
    let pb_file = mpb.add(ProgressBar::new(filesize));
    let pb_part = mpb.add(ProgressBar::new(parts_num));

    let (style_int, style_data) = progress_bars();

    pb_file.set_style(style_data);
    pb_part.set_style(style_int);

    let mut file = pb_file.wrap_read(file);
    for part in pb_part.wrap_iter(0..parts_num) {
        let response = discord::upload(
            &format!("chuncord_{part}"),
            (&mut file).take(MAXBUF),
            &webhook,
        )?;
        parts.push((response.id, response.attachment.url.into()));
    }

    pb_file.finish_and_clear();

    let index = Index {
        name: filename.into(),
        size: filesize,
        parts,
    };
    let index_toml = toml::to_string(&index)?;
    let stream = include_bytes!("index_header.toml").chain(index_toml.as_bytes());
    let response = discord::upload("chuncord_index.toml", stream, &webhook)?;
    println!(
        "Done!\nURL: {}\nMID (required for delete): {}",
        response.attachment.url, response.id
    );
    Ok(())
}

pub fn download(index_url: &str, path: Option<&str>) -> Result<()> {
    let index: Index = toml::from_str(&ureq::get(index_url).call()?.into_string()?)?;
    let path = if let Some(path) = path {
        let path = Path::new(path);
        if path.is_dir() {
            path.join(index.name.as_ref())
        } else {
            path.into()
        }
    } else {
        Path::new(index.name.as_ref()).into()
    };
    

    let mpb = MultiProgress::new();
    let pb_file = mpb.add(ProgressBar::new(index.size));
    let pb_part = mpb.add(ProgressBar::new(index.parts.len() as u64));

    let (style_int, style_data) = progress_bars();
    pb_file.set_style(style_data);
    pb_part.set_style(style_int);

    let mut file = pb_file.wrap_write(File::create(path)?);

    for (_, part) in pb_part.wrap_iter(index.parts.into_iter()) {
        let mut reader = ureq::get(&part).call()?.into_reader();
        io::copy(&mut reader, &mut file)?;
    }

    pb_file.finish_and_clear();

    println!("Done!");
    Ok(())
}

pub fn delete(mid: u64, webhook: Option<&str>) -> Result<()> {
    let webhook = get_webhook(webhook)?;
    let index_message_json = ureq::get(&format!("{webhook}/messages/{mid}"))
        .call()?
        .into_reader();
    let index_message: DiscordMessage = serde_json::from_reader(index_message_json)?;
    let index_url = index_message.attachment.url;
    let index_toml = ureq::get(&index_url).call()?.into_string()?;
    let index: Index = toml::from_str(&index_toml)?;

    let pb = ProgressBar::new(index.parts.len() as u64);
    pb.set_style(progress_bars().0);

    for (part, _) in pb.wrap_iter(index.parts.into_iter()) {
        discord::delete(part, &webhook)?;
    }

    println!("Deleting index...");
    discord::delete(mid, &webhook)?;
    println!("Done!");
    Ok(())
}

pub fn add_webhook(name: String, url: String) -> Result<()> {
    let mut cfg = Config::load()?;
    let set_default = cfg.webhooks.is_empty();
    let entry = cfg.webhooks.entry(name.clone());
    match entry {
        Entry::Occupied(_) => return Err(eyre!("Webhook {} already exists", entry.key())),
        Entry::Vacant(v) => {
            if set_default {
                cfg.default_webhook = Some(name);
            }
            v.insert(url);
        },
    };
    cfg.save()
}

pub fn del_webhook(name: &str) -> Result<()> {
    let mut cfg = Config::load()?;
    if cfg.webhooks.remove(name).is_none() {
        return Err(eyre!("Webhook '{name}' not found."))
    }
    cfg.save()
}

pub fn list_webhooks() -> Result<()> {
    let cfg = Config::load()?;
    for (name, url) in cfg.webhooks {
        let default = if cfg.default_webhook.as_deref() == Some(&name) {
            "[*] "
        } else {
            ""
        };
        println!(" - {default}{name} \x1b[90m({url})\x1b[0m");
    }
    Ok(())
}

pub fn default_webhook(name: String) -> Result<()> {
    let mut cfg = Config::load()?;
    if !cfg.webhooks.contains_key(&name) {
        return Err(eyre!("Webhook '{name}' not found"))
    }
    cfg.default_webhook = Some(name);
    cfg.save()
}
