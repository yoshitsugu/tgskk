use std::ffi::OsStr;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

pub type Stream = UnixStream;

pub fn spawn<I, S>(executable: &Path, args: I) -> io::Result<(Child, Stream)>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let (stream, child_stream) = UnixStream::pair()?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_millis(500)))?;
    let child = Command::new(executable)
        .args(args)
        .stdin(Stdio::from(OwnedFd::from(child_stream.try_clone()?)))
        .stdout(Stdio::from(OwnedFd::from(child_stream)))
        .stderr(Stdio::null())
        .spawn()?;
    Ok((child, stream))
}
