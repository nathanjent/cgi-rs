#[macro_use]
extern crate rouille;
extern crate chrono;
extern crate dotenv;
#[macro_use]
extern crate serde_derive;
extern crate envy;

use dotenv::dotenv;
use std::io::{self, Read, Write};
use std::panic;
use std::time::{Duration, Instant};
use rouille::Request;
use rouille::Response;

#[derive(Deserialize, Debug)]
struct EnvRequest {
    #[serde(rename = "REQUEST_METHOD")]
    request_method: String,
    #[serde(rename = "REQUEST_URI")]
    request_uri: String,
    #[serde(rename = "REMOTE_ADDR")]
    remote_addr: String,
    #[serde(rename = "REMOTE_PORT")]
    remote_port: u64,
    #[serde(rename = "CONTENT_LENGTH", default)]
    content_length: u64,
    #[serde(default = "http_headers")]
    headers: Vec<(String, String)>,
}

fn http_headers() -> Vec<(String, String)> {
    ::std::env::vars().filter_map(|(k, v)| {
       match k.split("HTTP_").nth(1) {
           Some(k) => Some((k.into(), v)),
           None => None,
       }
    }).collect::<Vec<_>>()
}

fn main() {
    dotenv().ok();
    //println!("{:?}", ::std::env::vars().collect::<Vec<_>>());

    let status = match handle() {
        Ok(_) => 0,
        Err(e) => {
            writeln!(io::stdout(), "Status: 500\r\n\r\n
                     <h1>500 Internal Server Error</h1>
                     <p>{}</p>", e)
                .expect("Panic at write to STDOUT!");
            1
        }
    };
    ::std::process::exit(status);
}

fn handle() -> Result<(), Box<::std::error::Error>> {
    // Deserialize request from environment variables
    let request = envy::from_env::<EnvRequest>()?;
    //println!("{:?}", request);

    // Read request body from stdin
    let mut data = Vec::new();
    io::stdin().take(request.content_length).read_to_end(&mut data)?;

    // Generate a Rouille Request
    let request =
        Request::fake_http_from(
            format!("{}:{}", request.remote_addr, request.remote_port).parse()?,
                                request.request_method,
                                request.request_uri,
                                request.headers,
                                data);

    // Route request 
    let _response = router!(request,
        // first route
        (GET) (/) => {
            let mut s = String::new();
            for (k, v) in request.headers() {
                s.push_str(&*format!("{}: {}\r\n", k, v));
            }
            Response::text(s)
        },

        // second route
        (GET) (/hello) => {
            Response::text("Hello")
        },

        // ... other routes here ...

        // default route
        _ => {
            Response::text("Default Space")
        }
    );

    // Send resulting response after routing
    send(&request, io::stdout(), || _response)?;
    Ok(())
}

fn send<W, F>(rq: &Request, mut output: W, f: F)
    -> Result<(), Box<::std::error::Error>>
    where W: Write,
          F: FnOnce() -> Response
{
    let start_instant = Instant::now();
    let rq_line = format!("{} UTC - {} {}",
                          chrono::UTC::now().format("%Y-%m-%d %H:%M:%S%.6f"),
                          rq.method(),
                          rq.raw_url());

    // Calling the handler and catching potential panics.
    // Note that this we always resume unwinding afterwards, we can ignore the small panic-safety
    // mecanism of `catch_unwind`.
    let response = panic::catch_unwind(panic::AssertUnwindSafe(f));

    let elapsed_time = format_time(start_instant.elapsed());

    match response {
        Ok(response) => {
            for &(ref k, ref v) in response.headers.iter() {
                writeln!(output, "{}: {}", k, v)?;
            }
            //writeln!(output, "Status: {}", response.status_code)?;
            let (mut response_body, content_length) = response.data.into_reader_and_size();
            if let Some(content_length) = content_length {
                writeln!(output, "Content-Length: {}",  content_length)?;
            }
            writeln!(output, "")?;
            io::copy(&mut response_body, &mut output)?;
            writeln!(output, "")?;
        }
        Err(payload) => {
            // There is probably no point in printing the payload, as this is done by the panic
            // handler.
            let _ = writeln!(output, "{} - {} - PANIC!", rq_line, elapsed_time);
            panic::resume_unwind(payload);
        }
    }
    Ok(())
}

fn format_time(duration: Duration) -> String {
    let secs_part = match duration.as_secs().checked_mul(1_000_000_000) {
        Some(v) => v,
        None => return format!("{}s", duration.as_secs() as f64),
    };

    let duration_in_ns = secs_part + duration.subsec_nanos() as u64;

    if duration_in_ns < 1_000 {
        format!("{}ns", duration_in_ns)
    } else if duration_in_ns < 1_000_000 {
        format!("{:.1}us", duration_in_ns as f64 / 1_000.0)
    } else if duration_in_ns < 1_000_000_000 {
        format!("{:.1}ms", duration_in_ns as f64 / 1_000_000.0)
    } else {
        format!("{:.1}s", duration_in_ns as f64 / 1_000_000_000.0)
    }
}
