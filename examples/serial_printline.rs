//! Simple example that echoes received serial traffic to stdout
extern crate mio;
extern crate mio_serial;

#[cfg(unix)]
use mio::unix::UnixReady;
use mio::{Events, Poll, PollOpt, Ready, Token};
use std::env;
use std::io;
use std::io::Read;
use std::str;

const SERIAL_TOKEN: Token = Token(0);

#[cfg(unix)]
const DEFAULT_TTY: &str = "/dev/ttyUSB0";
#[cfg(windows)]
const DEFAULT_TTY: &str = "COM1";

#[cfg(unix)]
fn ready_of_interest() -> Ready {
    Ready::readable() | UnixReady::hup() | UnixReady::error()
}

#[cfg(windows)]
fn ready_of_interest() -> Ready {
    Ready::readable()
}

#[cfg(unix)]
fn is_closed(state: Ready) -> bool {
    state.contains(UnixReady::hup() | UnixReady::error())
}

#[cfg(windows)]
fn is_closed(state: Ready) -> bool {
    false
}

pub fn main() {
    let mut args = env::args();
    let tty_path = args.nth(1).unwrap_or_else(|| DEFAULT_TTY.into());

    let poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1024);

    // Create the listener
    println!("Opening {} at 115200,8N1", tty_path);
    let settings = mio_serial::new(tty_path, 115200);
    let mut rx = mio_serial::Serial::from_builder(settings).unwrap();

    poll.register(&rx, SERIAL_TOKEN, ready_of_interest(), PollOpt::edge())
        .unwrap();

    let mut rx_buf = [0u8; 1024];

    'outer: loop {
        if let Err(ref e) = poll.poll(&mut events, None) {
            println!("poll failed: {}", e);
            break;
        }

        if events.is_empty() {
            println!("Read timed out!");
            continue;
        }

        for event in events.iter() {
            match event.token() {
                SERIAL_TOKEN => {
                    let ready = event.readiness();
                    if is_closed(ready) {
                        println!("Quitting due to event: {:?}", ready);
                        break 'outer;
                    }
                    if ready.is_readable() {
                        // With edge triggered events, we must perform reading until we receive a WouldBlock.
                        // See https://docs.rs/mio/0.6/mio/struct.Poll.html for details.
                        loop {
                            match rx.read(&mut rx_buf) {
                                Ok(count) => {
                                    println!("{:?}", String::from_utf8_lossy(&rx_buf[..count]))
                                }
                                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    break;
                                }
                                Err(ref e) => {
                                    println!("Quitting due to read error: {}", e);
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
                t => unreachable!("Unexpected token: {:?}", t),
            }
        }
    }
}
