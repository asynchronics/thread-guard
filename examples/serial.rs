//! This example demonstrates a thread guard that automatically joins the serial
//! port communication thread.
//!
//! In this example, a thread that reads data from a serial port and echoes it
//! back is used to illustrate the features of the thread guard.
//!
//! Before running the example, execute serial-setup.sh in another shell.

use std::io::{Error, ErrorKind, Read, Write};
use std::thread;
use std::time::Duration;

use mio::event::Source;
use mio::{Events, Interest, Poll, Token, Waker};
use mio_serial::SerialPortBuilderExt;

use thread_guard::ThreadGuard;

/// Echo port.
const PORT_PATH_A: &str = "/tmp/ttyS20";
/// Main thread port.
const PORT_PATH_B: &str = "/tmp/ttyS21";

/// Serial port readable token.
const SERIAL: Token = Token(0);
/// Thread exit token.
const WAKE: Token = Token(1);

/// Returns a thread guard for serial port echoing thread.
fn port_a() -> Result<ThreadGuard<Result<(), Error>, Waker>, Error> {
    let mut poll = Poll::new()?;
    let mut port = mio_serial::new(PORT_PATH_A, 0).open_native_async()?;
    let registry = poll.registry();
    port.register(registry, SERIAL, Interest::READABLE)?;
    let waker = Waker::new(registry, WAKE)?;
    let guard = ThreadGuard::with_actions(
        thread::spawn(move || {
            let mut events = Events::with_capacity(256);
            let mut buf = vec![0; 256];
            'poll: loop {
                // This call is blocking.
                poll.poll(&mut events, None).unwrap();

                for event in events.iter() {
                    let token = event.token();
                    // We are asked to exit.
                    if token == WAKE {
                        break 'poll;
                    } else {
                        // There is something to read.
                        loop {
                            match port.read(&mut buf) {
                                Ok(len) => {
                                    println!(
                                        "Port A received a message: {:?}, going to echo it.",
                                        &buf[..len]
                                    );
                                    if port.write(&buf[..len])? != len {
                                        return Err(Error::new(
                                            ErrorKind::Other,
                                            "Write did not fully succed.",
                                        ));
                                    };
                                }
                                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                    break;
                                }
                                Err(e) => return Err(e),
                            }
                        }
                    }
                }
            }
            Ok(())
        }),
        waker,
        |waker, _| {
            // Make thread exit. That's important to make sure that the waker is
            // alive until wake signal is delivered, that's why it is kept in
            // the guard, not in the closure.
            let _ = waker.wake();
        },
        |r| {
            println!("Port A thread exited with the result {:?}.", r);
        },
    );

    Ok(guard)
}

/// Sends data through serial port and checks the echo.
fn scenario_echo_join() -> Result<(), Error> {
    let port_a = port_a()?;
    let mut port_b = serialport::new(PORT_PATH_B, 0).open()?;

    // Send data.
    let message = vec![1, 2, 3, 4];
    assert_eq!(port_b.write(&message)?, message.len());

    // Read data back and check that it was echoed.
    let mut buf = vec![0, 0, 0, 0];
    port_b.set_timeout(Duration::from_secs(5))?;
    let rec_len = port_b.read(&mut buf)?;
    assert_eq!(rec_len, message.len());
    println!("Port B received a message {:?}.", &buf[..rec_len]);
    assert_eq!(message, buf);

    // Call join explicitly.
    match port_a.join() {
        Ok(r) => r?,
        Err(e) => panic!("Port A panic {:?}.", e),
    }

    Ok(())
}

/// Creates and drops the thread guard.
fn scenario_drop() -> Result<(), Error> {
    let _port_a = port_a()?;
    // Exit, pre-action and thread join are called automatically.
    Ok(())
}

/// Runs example scenarios.
fn main() -> Result<(), Error> {
    scenario_echo_join()?;
    scenario_drop()?;
    Ok(())
}
