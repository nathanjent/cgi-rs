/// This is an example CGI script application.
///
/// It prints the environment variables set by the host server.
use std::io::{self, Read, Write};
use std::env;

fn main() {
    let status = match handle() {
        Ok(_) => 0,
        Err(e) => {
            // Write out errors on exit
            writeln!(io::stdout(), "Status: 500\r\n\r\n
                <h1>Internal Server Error</h1>
                <p>{}</p>", e).unwrap();
            1
        }
    };
    ::std::process::exit(status);
}

fn handle() -> Result<(), Box<::std::error::Error>> {
    let content_length = env::var("CONTENT_LENGTH")
        .unwrap_or("0".into())
        .parse::<u64>()?;

    // Read from STDIN up to CONTENT_LENGTH
    let mut buffer = Vec::new();
    io::stdin().take(content_length).read_to_end(&mut buffer)?;

    // Response is written to STDOUT
    // macros for STOUT make it easier
    println!("Content-Type: text/html");
    println!();
    println!("<p>Hello, world!</p>");

    // write out environment as html list
    println!("<ul>");
    for (key, value) in ::std::env::vars() {
        println!("<li>{}: {}</li>", key, value);
    }
    println!("</ul>");

    // write out buffer from STDIN
    println!("<p>");
    io::stdout().write(&buffer[..])?;
    println!("</p>");

    Ok(())
}
