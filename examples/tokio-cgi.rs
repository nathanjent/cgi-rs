extern crate dotenv;
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
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let (read, write) = Stdio::new(1, 1).split();

    let mut buffer = Vec::new();
    let reader = tokio_core::io::read(read, buffer);
    
    //reader.and_then(|(read, body, length)| {
    //    tokio_core::io::write_all(write, body).wait().unwrap();
    //});
}
