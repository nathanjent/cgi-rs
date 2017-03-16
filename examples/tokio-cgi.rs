extern crate dotenv;
extern crate envy;
extern crate futures;
extern crate mio;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;
extern crate tokio_stdio;

use futures::{future, BoxFuture, Future};
use std::net::SocketAddr;
use std::sync::{Mutex, Arc};
use std::str;
use std::io::{self, Read, Write};
use std::env;
use std::collections::HashMap;
use tokio_core::io::{Codec, EasyBuf, Io, Framed};
use tokio_core::reactor::{Core, PollEvented};
use tokio_proto::BindServer;
use tokio_proto::pipeline::ServerProto;
use tokio_service::{NewService, Service};
use tokio_stdio::stdio::Stdio;
use mio::{Evented, Ready, Poll, PollOpt, Token};

use tokio_proto::TcpServer;

fn main() {
    dotenv::dotenv().ok();

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let stdio = Stdio::new(1, 1);
    let server = CgiProto;
    let service = CgiService;
    server.bind_server(&handle, stdio, service);

    //let (read, write) = Stdio::new(1, 1).split();

    //let mut buffer = Vec::new();
    //let responder = tokio_core::io::read_to_end(read, buffer)
    //    .and_then(|(read, body)| {
    //        tokio_core::io::write_all(write, body)
    //    });

    //let status = match core.run(responder) {
    //    Ok(_) => 0,
    //    Err(_) => 1,
    //};
    //::std::process::exit(status);
}

#[derive(Debug)]
enum CgiRequest {
    Get { url: String },
    Post { url: String, content: String },
}

#[derive(Debug)]
enum CgiResponse {
    NotFound,
    Ok { content: String },
}

#[derive(Default)]
struct CgiService;
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

        // Return the appropriate value.
        let res = match req {
            CgiRequest::Get { url: url } => {
                CgiResponse::Ok { content: "GET".into() }
            }
            CgiRequest::Post { url: url, content: content } => {
                CgiResponse::Ok { content: "POST".into() }
            }
        };

        // Return the result.
        future::finished(res).boxed()
    }
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

        let mut url = None;
        let mut method = None;

        // get request fields from headers set as environment variables
        for (k, v) in env::vars() {
            match &*k {
                "HTTP_REQUEST_METHOD" => {
                    method = Some(v);
                }
                "HTTP_REQUEST_URL" => {
                    url = Some(v);
                }
                _ => {}
            }
        }

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
                Ok(Some(CgiRequest::Get {
                    url: url.unwrap().into(),
                }))
            }
            Some(ref method) if method == "POST" => {
                Ok(Some(CgiRequest::Post {
                    url: url.unwrap().into(),
                    content: content,
                }))
            }
            _ => Err(io::Error::new(io::ErrorKind::Other, "invalid")),
        }
    }

    // Produces a frame.
    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> io::Result<()> {
        match msg {
            CgiResponse::NotFound => {
                buf.extend(b"HTTP/1.1 404 Not Found\r\n");
                buf.extend(b"Content-Length: 0\r\n");
                buf.extend(b"Connection: close\r\n");
            }
            CgiResponse::Ok { content: v } => {
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
