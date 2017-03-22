extern crate futures;
extern crate tokio_core;
extern crate tokio_stdio;

use futures::{future, Future};
use tokio_stdio::stdio::Stdio;
use tokio_core::io::{Codec, EasyBuf, Io, Framed};
use tokio_core::reactor::{Handle, Core};

pub fn main() {
    let mut core = Core::new().unwrap();
    let buf = Vec::new();
    let (read, write) = Stdio::new(1, 1).split();
    let server = future::lazy(move || {
        tokio_core::io::read_to_end(read, buf).and_then(|(read, buf)| {
            tokio_core::io::write_all(write, buf.into_iter().rev().collect::<Vec<_>>())
        })
    });

    ::std::process::exit(match core.run(server) {
        Ok(_) => 0,
        Err(_) => 1,
    });
}
