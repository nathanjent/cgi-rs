use std::io::{self, Read, Write};
use std::env;

fn main() {
    let content_length = env::var("CONTENT_LENGTH")
        .unwrap_or("0".into())
        .parse::<u64>()
        .expect("Error parsing CONTENT_LENGTH");
    let status = match handle(content_length) {
        Ok(_) => 0,
        Err(_) => 1,
    };
    ::std::process::exit(status);
}

fn handle(content_length: u64) -> io::Result<()> {
    let mut buffer = Vec::new();
    io::stdin().take(content_length).read_to_end(&mut buffer)?;

    println!("Content-Type: text/html");
    println!();
    println!("<p>Hello, world!</p>");
    println!("<ul>");
    for (key, value) in ::std::env::vars() {
        println!("<li>{}: {}</li>", key, value);
    }
    println!("</ul>");

    println!("<p>");
    io::stdout().write(&buffer[..])?;
    println!("</p>");

    Ok(())
}
