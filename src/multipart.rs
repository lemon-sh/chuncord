use std::io::{Cursor, Read};

use rand::{thread_rng, RngCore};

pub fn multipart<R>(file: R, name: &str) -> (String, impl Read)
where
    R: Read,
{
    let boundary = thread_rng().next_u64();
    let content_type = format!("multipart/form-data; boundary={boundary}");

    let fd_head = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n").into_bytes();
    let fd_tail = format!("\r\n--{boundary}--\r\n").into_bytes();

    let stream = Cursor::new(fd_head).chain(file).chain(Cursor::new(fd_tail));

    (content_type, stream)
}
