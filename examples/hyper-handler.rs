extern crate dotenv;
extern crate envy;
extern crate hyper_serde;
extern crate hyper_router;

use dotenv::dotenv;

use std::io::{self, Read, Write};
use std::env;

use cookie::Cookie;
use hyper::header::{ContentType, Headers};
use hyper::http::RawStatus;
use hyper::method::Method;
use hyper_serde::{De, Ser, deserialize};
use serde::Deserialize;
use serde_test::{Deserializer, Token, assert_ser_tokens};
use std::fmt::Debug;
use time::Duration;

#[derive(Deserialize, Debug)]
struct CgiRequest {
    #[serde(deserialize_with = "hyper_serde::deserialize",
            serialize_with = "hyper_serde::serialize")]
    cookie: Cookie,
    #[serde(deserialize_with = "hyper_serde::deserialize",
            serialize_with = "hyper_serde::serialize")]
    content_type: ContentType,
    #[serde(deserialize_with = "hyper_serde::deserialize",
            serialize_with = "hyper_serde::serialize")]
    headers: Headers,
    #[serde(deserialize_with = "hyper_serde::deserialize",
            serialize_with = "hyper_serde::serialize")]
    raw_status: RawStatus,
    #[serde(deserialize_with = "hyper_serde::deserialize",
            serialize_with = "hyper_serde::serialize")]
    method: Method,
}

fn main() {
    dotenv::dotenv().ok();

    let status = match handle() {
        Ok(_) => 0,
        Err(_) => 1,
    };
    ::std::process::exit(status);
}

fn handle() -> io::Result<()> {
    let content_length = env::var("CONTENT_LENGTH").unwrap_or("0".into())
        .parse::<u64>().expect("Error parsing CONTENT_LENGTH");
    let mut buffer = Vec::new();
    io::stdin().take(content_length).read_to_end(&mut buffer)?;

    // TODO parse env vars into stream
    let buffer: Vec<u8> = env::vars()
        .filter_map(|(k, v)| {
            if k.starts_with("HTTP_") {
                if let Some(header) = k.splitn(2, '_').nth(1) {
                    header.replace('_', "-");
                    Some(format!("{}: {}\r\n\r\n", header, v))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .flat_map(String::into_bytes)
        .chain(buffer.into_iter()).collect();


    io::stdout().write(&buffer[..])?;

    Ok(())
}
