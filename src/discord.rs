use std::{thread::sleep, time::Duration};

use color_eyre::Result;
use multipart::client::lazy::Multipart;
use serde::{Deserialize, Deserializer};
use ureq::Response;

fn singleton<'de, D, T>(v: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let [item] = <[T; 1] as Deserialize>::deserialize(v)?;
    Ok(item)
}

#[derive(Deserialize)]
pub struct DiscordMessage {
    pub id: u64,
    #[serde(rename = "attachments")]
    #[serde(deserialize_with = "singleton")]
    pub attachment: Attachment,
}

#[derive(Deserialize)]
pub struct Attachment {
    pub url: String,
}

pub fn upload(filename: String, data: &[u8], webhook: &str) -> Result<DiscordMessage> {
    let mut mp = Multipart::new();
    mp.add_stream("file", data, Some(filename), None);
    let mpdata = mp.prepare()?;
    let response = ureq::post(webhook)
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={}", mpdata.boundary()),
        )
        .send(mpdata)?;
    cool_ratelimit(&response)?;
    Ok(serde_json::from_reader(response.into_reader())?)
}

pub fn delete(mid: u64, webhook: &str) -> Result<()> {
    let response = ureq::delete(&format!("{webhook}/messages/{mid}")).call()?;
    cool_ratelimit(&response)
}

fn cool_ratelimit(resp: &Response) -> Result<()> {
    let rt_header: u64 = resp.header("X-RateLimit-Remaining").unwrap().parse()?;
    if rt_header == 0 {
        let rt_reset = resp.header("X-RateLimit-Reset-After").unwrap();
        let sleep_seconds: f64 = rt_reset.parse()?;
        sleep(Duration::from_secs_f64(sleep_seconds));
    }
    Ok(())
}
