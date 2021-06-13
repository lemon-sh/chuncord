use clap::{App, Arg};
use std::process;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Seek, SeekFrom, Read, Write};
use multipart::client::lazy::Multipart;
use json::{object};
use std::path::Path;
use std::fmt;
use std::fmt::Formatter;
use std::thread::sleep;
use std::time::Duration;

const MAXBUF: u64 = 8380416;

impl Error for ChuncordError {}

impl fmt::Display for ChuncordError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ChuncordError::InvalidIndexJson(j) => write!(f, "Index JSON is invalid: {}", j),
            ChuncordError::InvalidApiJson(j) => write!(f, "Discord JSON is invalid: {}", j),
            ChuncordError::FileTooBig => write!(f, "File is too big")
        }
    }
}

#[derive(Debug)]
enum ChuncordError {
    InvalidIndexJson(String),
    InvalidApiJson(String),
    FileTooBig
}

fn upload_discord(filename: &str, data: &[u8], webhook: &str) -> Result<(String, String), Box<dyn Error>> {
    let mut mp = Multipart::new();
    mp.add_stream("file", data, Some(filename.to_string()), None);
    let mpdata = mp.prepare()?;
    let response = ureq::post(webhook)
        .set("Content-Type", &format!("multipart/form-data; boundary={}", mpdata.boundary()))
        .send(mpdata)?;
    let response_string = response.into_string()?;
    let mut rjson = json::parse(&response_string)?;
    let url = rjson["attachments"][0]["url"].take_string().ok_or_else(|| ChuncordError::InvalidApiJson(response_string.clone()))?;
    let id = rjson["id"].take_string().ok_or_else(|| ChuncordError::InvalidApiJson(response_string.clone()))?;
    Ok((url, id))
}

fn delete_discord(mid: &str, webhook: &str) -> Result<(), Box<dyn Error>> {
    match ureq::delete(&format!("{}/messages/{}", webhook, mid)).call() {
        Err(ureq::Error::Status(429, response)) => {
            let ratelimit_raw = response.into_string()?;
            let ratelimit_json = json::parse(&ratelimit_raw)?;
            let retry_after = ratelimit_json["retry_after"].as_u64().ok_or(ChuncordError::InvalidIndexJson(ratelimit_raw.clone()))?;
            sleep(Duration::from_millis(retry_after));
            delete_discord(mid, webhook)
        }
        Err(e) => Err(Box::new(e)),
        Ok(_) => Ok(())
    }
}

fn download_file(url: &str) -> Result<impl Read + Send, Box<dyn Error>> {
    Ok(ureq::get(url).call()?.into_reader())
}

fn download_index(url: &str) -> Result<String, Box<dyn Error>> {
    Ok(ureq::get(url).call()?.into_string()?)
}

fn upload_command(file: &str, webhook: &str) -> Result<(), Box<dyn Error>> {
    println!("Analyzing file...");
    let filename = Path::new(file).file_name().unwrap().to_str().unwrap();
    let mut file = File::open(file)?;
    let filesize = file.seek(SeekFrom::End(0))?;
    if filesize > u32::MAX as u64 {
        return Err(Box::new(ChuncordError::FileTooBig));
    }
    let fullreads = filesize / MAXBUF;
    let lastread = filesize % MAXBUF;
    file.seek(SeekFrom::Start(0))?;
    let mut index_json = object! {name: filename, parts: {}};
    let mut buffer = vec![0 as u8; MAXBUF as usize];
    for fullread in 0..fullreads {
        print!("Uploading... [{}/{}]\r", fullread, fullreads);
        io::stdout().flush()?;
        file.read_exact(&mut buffer)?;
        let upload_result = upload_discord(fullread.to_string().as_str(), &buffer, webhook)?;
        index_json["parts"].insert(upload_result.0.as_str(), upload_result.1.as_str())?;
    }
    print!("Uploading... [{0}/{0}]\r", fullreads);
    io::stdout().flush()?;
    if lastread > 0 {
        file.read_exact(&mut buffer[0..lastread as usize])?;
        let upload_result = upload_discord(fullreads.to_string().as_str(), &buffer[0..lastread as usize], webhook)?;
        index_json["parts"].insert(upload_result.0.as_str(), upload_result.1.as_str())?;
    }
    println!("\nUploading index...");
    let index_upload_result = upload_discord("index.json", index_json.dump().as_bytes(), webhook)?;
    println!("\nDone!\nIndex URL: {}\nIndex message ID (needed for delete): {}", index_upload_result.0, index_upload_result.1);
    Ok(())
}

fn download_command(file: Option<&str>, index_url: &str) -> Result<(), Box<dyn Error>> {
    println!("Downloading index...");
    let index_json = download_index(index_url)?;
    let mut parsed_index = json::parse(&index_json)?;
    let parts_count = parsed_index["parts"].len();
    if parts_count == 0 {
        return Err(Box::new(ChuncordError::InvalidIndexJson(index_json)));
    }
    let index_filename = parsed_index["name"].take_string().ok_or(ChuncordError::InvalidIndexJson(index_json))?;
    let filename = file.unwrap_or(index_filename.as_str());
    let mut file = OpenOptions::new().write(true).create_new(true).open(filename)?;
    let parts = parsed_index["parts"].entries();
    for part in (0..).zip(parts) {
        print!("Downloading... [{}/{}]\r", part.0, parts_count-1);
        io::stdout().flush()?;
        let mut downloaded_part = download_file(part.1.0)?;
        io::copy(&mut downloaded_part, &mut file)?;
    }
    println!("\nDone!");
    Ok(())
}

fn delete_command(mid: &str, webhook: &str) -> Result<(), Box<dyn Error>> {
    println!("Downloading index...");
    let index_message_result = ureq::get(&format!("{}/messages/{}", webhook, mid)).call()?.into_string()?;
    let mut index_message_result_json = json::parse(index_message_result.as_str())?;
    let index_url = index_message_result_json["attachments"][0]["url"].take_string()
        .ok_or(ChuncordError::InvalidApiJson(index_message_result))?;
    let index = download_index(&index_url)?;
    let mut index_json = json::parse(&index)?;
    let parts_count = index_json["parts"].len();
    if parts_count == 0 {
        return Err(Box::new(ChuncordError::InvalidIndexJson(index_url)));
    }
    let parts = index_json["parts"].entries_mut();
    for part in (0..).zip(parts) {
        print!("Deleting... [{}/{}]\r", part.0, parts_count-1);
        io::stdout().flush()?;
        delete_discord(&part.1.1.take_string().ok_or_else(|| ChuncordError::InvalidIndexJson(index.clone()))?, webhook)?;
    }
    println!("\nDeleting index...");
    delete_discord(mid, webhook)?;
    println!("\nDone!");
    Ok(())
}

fn main() {
    let clapmatch = App::new("Chuncord").version("0.1").author("by ./lemon.sh")
        .about("Upload chunky files to Discord with Webhooks")
        .subcommand(App::new("upload").about("Upload file")
            .arg(Arg::new("webhook").required(true).short('w').takes_value(true).about("Discord Webhook"))
            .arg(Arg::new("file").required(true).short('f').takes_value(true).about("File to upload")))
        .subcommand(App::new("download").about("Download file")
            .arg(Arg::new("url").short('u').required(true).about("Index URL").takes_value(true))
            .arg(Arg::new("file").short('o').about("Output file").takes_value(true)))
        .subcommand(App::new("delete").about("Delete file")
            .arg(Arg::new("mid").short('m').required(true).about("Index message ID").takes_value(true))
            .arg(Arg::new("webhook").required(true).short('w').takes_value(true).about("Discord Webhook")))
        .get_matches();
    let subcommand = clapmatch.subcommand().unwrap_or_else(|| {
        println!("No subcommand provided. See --help");
        process::exit(1)
    });
    if let Err(e) = match subcommand.0 {
        "upload" => upload_command(subcommand.1.value_of("file").unwrap(), subcommand.1.value_of("webhook").unwrap()),
        "download" => download_command(subcommand.1.value_of("file"), subcommand.1.value_of("url").unwrap()),
        "delete" => delete_command(subcommand.1.value_of("mid").unwrap(), subcommand.1.value_of("webhook").unwrap()),
        _ => Ok(())
    } {
        println!("An error has occurred.\n{}", e);
    }
}
