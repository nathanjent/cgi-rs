extern crate dotenv;
extern crate futures;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;

use dotenv::dotenv;
use futures::future::{self, BoxFuture, Future};
use std::net::SocketAddr;
use std::sync::{Mutex, Arc};
use std::str;
use std::io::{self, Read, Write};
use std::env;
use std::collections::HashMap;
use tokio_core::io::{Codec, EasyBuf, Io, Framed};
use tokio_core::reactor::{Core,};
use tokio_proto::TcpServer;
use tokio_proto::pipeline::ServerProto;
use tokio_service::{NewService, Service};

#[derive(Debug)]
enum CgiRequest {
    Get { url: String },
    Post { url: String, content: String },
}

enum CgiResponse {
    NotFound,
    Ok { content: String },
}

// Codecs can have state, but this one doesn't.
struct CgiCodec;
impl Codec for CgiCodec {
    // Associated types which define the data taken/produced by the codec.
    type In = CgiRequest;
    type Out = CgiResponse;

    // Returns `Ok(Some(In))` if there is a frame, `Ok(None)` if it needs more data.
    fn decode(&mut self, buf: &mut EasyBuf) -> io::Result<Option<Self::In>> {
        let content_so_far = String::from_utf8_lossy(buf.as_slice())
            .to_mut()
            .clone();

        // Request headers/content in HTTP 1.1 are split by double newline.
        match content_so_far.find("\r\n\r\n") {
            Some(i) => {
                // Drain from the buffer (this is important!)
                let mut headers = {
                    let tmp = buf.drain_to(i);
                    String::from_utf8_lossy(tmp.as_slice())
                        .to_mut()
                        .clone()
                };
                buf.drain_to(4); // Also remove the '\r\n\r\n'.

                // Get the method and drain.
                let method = headers.find(" ")
                    .map(|len| headers.drain(..len).collect::<String>());

                headers.drain(..1); // Get rid of the space.

                // Since the method was drained we can do it again to get the url.
                let url = headers.find(" ")
                    .map(|len| headers.drain(..len).collect::<String>())
                    .unwrap_or_default();

                // The content of a POST.
                let content = {
                    let remaining = buf.len();
                    let tmp = buf.drain_to(remaining);
                    String::from_utf8_lossy(tmp.as_slice())
                        .to_mut()
                        .clone()
                };

                match method {
                    Some(ref method) if method == "GET" => {
                        Ok(Some(CgiRequest::Get { url: url }))
                    },
                    Some(ref method) if method == "POST" => {
                        Ok(Some(CgiRequest::Post { url: url, content: content }))
                    },
                    _ => Err(io::Error::new(io::ErrorKind::Other, "invalid"))
                }
            },
            None => Ok(None),
        }
    }

    // Produces a frame.
    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> io::Result<()> {
        match msg {
            CgiResponse::NotFound => {
                buf.extend(b"HTTP/1.1 404 Not Found\r\n");
                buf.extend(b"Content-Length: 0\r\n");
                buf.extend(b"Connection: close\r\n");
            },
            CgiResponse::Ok { content: v } =>  {
                buf.extend(b"HTTP/1.1 200 Ok\r\n");
                buf.extend(format!("Content-Length: {}\r\n", v.len()).as_bytes());
                buf.extend(b"Connection: close\r\n");
                buf.extend(b"\r\n");
                buf.extend(v.as_bytes());
            }
        }
        buf.extend(b"\r\n");
        Ok(())
    }
}

// Like codecs, protocols can carry state too!
struct CgiProto;
impl<T: Io + 'static> ServerProto<T> for CgiProto {
    // These types must match the corresponding codec types:
    type Request = <CgiCodec as Codec>::In;
    type Response = <CgiCodec as Codec>::Out;

    /// A bit of boilerplate to hook in the codec:
    type Transport = Framed<T, CgiCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;
    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(CgiCodec))
    }
}

// Surprise! Services can also carry state.
#[derive(Default)]
struct CgiService {
    db: Arc<Mutex<HashMap<String, String>>>,
}

impl Service for CgiService {
    // These types must match the corresponding protocol types:
    type Request = <CgiCodec as Codec>::In;
    type Response = <CgiCodec as Codec>::Out;

    // For non-streaming protocols, service errors are always io::Error
    type Error = io::Error;

    // The future for computing the response; box it for simplicity.
    type Future = BoxFuture<Self::Response, Self::Error>;

    // Produce a future for computing a response from a request.
    fn call(&self, req: Self::Request) -> Self::Future {
        println!("Request: {:?}", req);
        
        // Deref the database.
        let mut db = self.db.lock()
            .unwrap(); // This should only panic in extreme cirumstances.
        
        // Return the appropriate value.
        let res = match req {
            CgiRequest::Get { url: url } => {
                match db.get(&url) {
                    Some(v) => CgiResponse::Ok { content: v.clone() },
                    None => CgiResponse::NotFound,
                }
            },
            CgiRequest::Post { url: url, content: content } => {
                match db.insert(url, content) {
                    Some(v) => CgiResponse::Ok { content: v },
                    None => CgiResponse::Ok { content: "".into() },
                }
            }
        };
        println!("Database: {:?}", *db);

        // Return the result.
        future::finished(res).boxed()
    }
}

fn main() {
    dotenv::dotenv().ok();
    
    
//    let socket: SocketAddr = LISTEN_TO.parse() .unwrap();
    
    // Create a server with the protocol.
//    let server = TcpServer::new(CgiProto, socket);

    // Create a database instance to provide to spawned services.
    let db = Arc::new(Mutex::new(HashMap::new()));

    // Serve requests with our created service and a handle to the database.    
//    server.serve(move || Ok(CgiService { db: db.clone() }));
    let status = match serve(move || Ok(CgiService { db: db.clone()})) {
        Ok(_) => 0,
        Err(_) => 1,
    };
    ::std::process::exit(status);
}

fn serve<S>(s: S) -> io::Result<()>
    where S: NewService<Request = CgiRequest,
                        Response = CgiResponse,
                        Error = io::Error> + 'static
{
    let mut core = Core::new()?;
    let handle = core.handle();


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

    let buffer = EasyBuf::from(buffer);
    //handle.spawn(move || {
    //    let p = CgiProto;
    //    p.bind_transport(buffer)
    //});

    io::stdout().write(buffer.as_slice())?;

    Ok(())
}
