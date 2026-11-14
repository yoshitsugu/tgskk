//! Bounded Windows stdio transport for managed helper processes.
//! The owner must kill and reap the child before dropping this transport, so
//! blocked OS pipe reads/writes are released as well as channel waits.
use std::cell::Cell;
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::Duration;

pub struct Pipe {
    incoming: Receiver<io::Result<Vec<u8>>>,
    outgoing: SyncSender<Vec<u8>>,
    written: Receiver<io::Result<()>>,
    buffer: Vec<u8>,
    position: usize,
    eof: bool,
    failed: bool,
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
            .name("tgskk-stdio-read".into())
            .spawn(move || {
                loop {
                    let mut bytes = vec![0; 1024];
                    let message = match stdout.read(&mut bytes) {
                        Ok(0) => Ok(Vec::new()),
                        Ok(n) => {
                            bytes.truncate(n);
                            Ok(bytes)
                        }
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Err(error) => Err(error),
                    };
                    let failed = message.as_ref().map_or(true, Vec::is_empty);
                    if send.send(message).is_err() || failed {
                        break;
                    }
                }
            })?;
        let (outgoing, receive) = mpsc::sync_channel::<Vec<u8>>(1);
        let (acknowledge, written) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("tgskk-stdio-write".into())
            .spawn(move || {
                while let Ok(bytes) = receive.recv() {
                    let result = stdin.write_all(&bytes).and_then(|()| stdin.flush());
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
            eof: false,
            failed: false,
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
fn failed_pipe() -> io::Error {
    io::Error::new(
        io::ErrorKind::BrokenPipe,
        "Worker pipe failed; discard this transport",
    )
}
impl Read for Pipe {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.failed {
            return Err(failed_pipe());
        }
        if out.is_empty() || self.eof {
            return Ok(0);
        }
        if self.position == self.buffer.len() {
            match self
                .incoming
                .recv_timeout(self.timeout.get())
                .map_err(receive_error)
            {
                Ok(Ok(bytes)) => self.buffer = bytes,
                Ok(Err(error)) | Err(error) => {
                    self.failed = true;
                    return Err(error);
                }
            }
            self.position = 0;
            if self.buffer.is_empty() {
                self.eof = true;
                return Ok(0);
            }
        }
        let count = out.len().min(self.buffer.len() - self.position);
        out[..count].copy_from_slice(&self.buffer[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}
impl Write for Pipe {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.failed || self.eof {
            return Err(failed_pipe());
        }
        if bytes.is_empty() {
            return Ok(0);
        }
        // A late acknowledgement after timeout belongs to the failed frame.
        self.failed = true;
        self.outgoing.try_send(bytes.to_vec()).map_err(|_| {
            io::Error::new(io::ErrorKind::BrokenPipe, "Worker pipe writer unavailable")
        })?;
        self.written
            .recv_timeout(self.write_timeout.get())
            .map_err(receive_error)??;
        self.failed = false;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Starts a console-free helper with piped stdio. The returned child
/// remains owned by the caller, including kill/reap on protocol failure.
pub fn spawn<I, S>(executable: &Path, args: I) -> io::Result<(Child, Pipe)>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut child = Command::new(executable)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    match Pipe::new(&mut child) {
        Ok(pipe) => Ok((child, pipe)),
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(error)
        }
    }
}
