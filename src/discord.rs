use std::{io::Read, thread::sleep, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Deserializer};
use ureq::Response;

use crate::multipart::multipart;

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
    #[serde(with = "string")]
    pub id: u64,
    #[serde(rename = "attachments")]
    #[serde(deserialize_with = "singleton")]
    pub attachment: Attachment,
}

#[derive(Deserialize)]
pub struct Attachment {
    pub url: String,
}

// https://github.com/serde-rs/json/issues/329#issuecomment-305608405
mod string {
    use serde::{de, Deserialize, Deserializer};
    use std::{fmt::Display, str::FromStr};

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

pub fn upload<R: Read>(filename: &str, stream: R, webhook: &str) -> Result<DiscordMessage> {
    let (mp_content_type, mp_stream) = multipart(stream, filename);
    let response = ureq::post(webhook)
        .set("Content-Type", &mp_content_type)
        .send(mp_stream)?;
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
