//! Bounded Windows stdio transport for managed helper processes.
//! The owner must kill and reap the child before dropping this transport, so
//! blocked OS pipe reads/writes are released as well as channel waits.
#![cfg(windows)]
use std::cell::Cell;
use std::io::{self, Read, Write};
use std::process::Child;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::Duration;

pub struct Pipe {
    incoming: Receiver<io::Result<Vec<u8>>>,
    outgoing: SyncSender<Vec<u8>>,
    written: Receiver<io::Result<()>>,
    buffer: Vec<u8>,
    position: usize,
    timeout: Cell<Duration>,
    write_timeout: Cell<Duration>,
}
impl Pipe {
    pub fn new(child: &mut Child) -> io::Result<Self> {
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("Missing worker stdout"))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("Missing worker stdin"))?;
        let (send, incoming) = mpsc::sync_channel(2);
        std::thread::Builder::new()
            .name("zenz-pipe-read".into())
            .spawn(move || {
                loop {
                    let mut bytes = vec![0; 1024];
                    let message = match stdout.read(&mut bytes) {
                        Ok(0) => Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Worker closed stdout",
                        )),
                        Ok(n) => {
                            bytes.truncate(n);
                            Ok(bytes)
                        }
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Err(error) => Err(error),
                    };
                    let failed = message.is_err();
                    if send.send(message).is_err() || failed {
                        break;
                    }
                }
            })?;
        let (outgoing, receive) = mpsc::sync_channel::<Vec<u8>>(1);
        let (acknowledge, written) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("zenz-pipe-write".into())
            .spawn(move || {
                while let Ok(bytes) = receive.recv() {
                    let result = stdin.write_all(&bytes).and_then(|_| stdin.flush());
                    let failed = result.is_err();
                    if acknowledge.send(result).is_err() || failed {
                        break;
                    }
                }
            })?;
        Ok(Self {
            incoming,
            outgoing,
            written,
            buffer: Vec::new(),
            position: 0,
            timeout: Cell::new(Duration::from_secs(5)),
            write_timeout: Cell::new(Duration::from_millis(500)),
        })
    }
    pub fn set_read_timeout(&self, duration: Option<Duration>) -> io::Result<()> {
        self.timeout
            .set(duration.ok_or_else(|| io::Error::other("Unbounded pipe reads are unsupported"))?);
        Ok(())
    }
    pub fn set_write_timeout(&self, duration: Option<Duration>) -> io::Result<()> {
        self.write_timeout.set(
            duration.ok_or_else(|| io::Error::other("Unbounded pipe writes are unsupported"))?,
        );
        Ok(())
    }
}
fn receive_error(error: mpsc::RecvTimeoutError) -> io::Error {
    match error {
        mpsc::RecvTimeoutError::Timeout => {
            io::Error::new(io::ErrorKind::TimedOut, "Worker pipe timed out")
        }
        mpsc::RecvTimeoutError::Disconnected => {
            io::Error::new(io::ErrorKind::BrokenPipe, "Worker pipe disconnected")
        }
    }
}
impl Read for Pipe {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        if self.position == self.buffer.len() {
            self.buffer = self
                .incoming
                .recv_timeout(self.timeout.get())
                .map_err(receive_error)??;
            self.position = 0;
        }
        let count = out.len().min(self.buffer.len() - self.position);
        out[..count].copy_from_slice(&self.buffer[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}
impl Write for Pipe {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        self.outgoing.try_send(bytes.to_vec()).map_err(|_| {
            io::Error::new(io::ErrorKind::BrokenPipe, "Worker pipe writer unavailable")
        })?;
        self.written
            .recv_timeout(self.write_timeout.get())
            .map_err(receive_error)??;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
