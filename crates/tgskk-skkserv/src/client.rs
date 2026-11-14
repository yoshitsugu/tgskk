//! Bounded synchronous exchange for SKK server transports.
//!
//! [`fetch`] performs exactly one TCP connection and one request/response
//! exchange against a numeric [`SocketAddr`]. The crate also reuses the shared
//! framing code for the managed helper's persistent Unix socket. No DNS is
//! done here, and callers only ever wait for a bounded deadline.

use std::error::Error as StdError;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use crate::{Candidate, Encoding, Error, MAX_RESPONSE_BYTES, decode_response, encode_request};

/// Failure of a bounded SKK server lookup.
#[derive(Debug)]
pub enum ClientError {
    /// The absolute deadline expired before or during the exchange.
    Timeout,
    /// Socket-level failure.
    Io(io::Error),
    /// The reading could not be encoded with the requested encoding.
    Encoding(Error),
    /// The response was not a valid SKK server response.
    Protocol(ProtocolError),
}

/// Protocol-level failure details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    /// The server reported an error with a leading `0`.
    ServerError,
    /// The connection closed before the response frame was complete.
    Truncated,
    /// The response exceeded [`MAX_RESPONSE_BYTES`].
    Oversized,
    /// A persistent transport delivered bytes after the frame terminator.
    TrailingData,
    /// The response failed codec validation.
    Codec(Error),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => f.write_str("SKK server lookup timed out"),
            Self::Io(_) => f.write_str("SKK server socket error"),
            Self::Encoding(err) => write!(f, "SKK lookup encoding error: {err}"),
            Self::Protocol(err) => write!(f, "SKK server protocol error: {err}"),
        }
    }
}

impl StdError for ClientError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Encoding(err) => Some(err),
            Self::Protocol(err) => Some(err),
            Self::Timeout => None,
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ServerError => f.write_str("SKK server reported an error"),
            Self::Truncated => {
                f.write_str("SKK server closed the connection before completing the response")
            }
            Self::Oversized => f.write_str("SKK server response is too large"),
            Self::TrailingData => {
                f.write_str("SKK server response contained unexpected trailing data")
            }
            Self::Codec(err) => write!(f, "{err}"),
        }
    }
}

impl StdError for ProtocolError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Codec(err) => Some(err),
            Self::ServerError | Self::Truncated | Self::Oversized | Self::TrailingData => None,
        }
    }
}

/// Looks up `reading` on the SKK server at `addr`.
///
/// The whole exchange (connect, write, read, including trickled responses) is
/// bounded by `deadline`. One fresh TCP connection is used per call and it is
/// closed before returning. The response frame is read into a bounded buffer
/// and validated by [`decode_response`].
pub fn fetch(
    addr: SocketAddr,
    reading: &str,
    encoding: Encoding,
    deadline: Instant,
) -> Result<Vec<Candidate>, ClientError> {
    remaining(deadline)?;

    let request = encode_request(reading, encoding).map_err(ClientError::Encoding)?;

    let connect_timeout = remaining(deadline)?;
    let mut stream = TcpStream::connect_timeout(&addr, connect_timeout).map_err(map_io)?;

    let result = exchange(
        &mut stream,
        &request,
        encoding,
        deadline,
        TrailingData::Allow,
    );
    drop(stream);
    result
}

/// Whether bytes after a frame's LF terminator are tolerated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrailingData {
    /// Fresh connections: ignore bytes after LF (existing TCP behaviour).
    Allow,
    /// Persistent sockets: reject any bytes after LF in the same read.
    Reject,
}

/// Socket types usable for one bounded SKK exchange.
pub(crate) trait DeadlineStream: Read + Write {
    fn arm_read(&self, timeout: Duration) -> io::Result<()>;
    fn arm_write(&self, timeout: Duration) -> io::Result<()>;
}

impl DeadlineStream for TcpStream {
    fn arm_read(&self, timeout: Duration) -> io::Result<()> {
        TcpStream::set_read_timeout(self, Some(timeout))
    }

    fn arm_write(&self, timeout: Duration) -> io::Result<()> {
        TcpStream::set_write_timeout(self, Some(timeout))
    }
}

#[cfg(unix)]
impl DeadlineStream for std::os::unix::net::UnixStream {
    fn arm_read(&self, timeout: Duration) -> io::Result<()> {
        std::os::unix::net::UnixStream::set_read_timeout(self, Some(timeout))
    }

    fn arm_write(&self, timeout: Duration) -> io::Result<()> {
        std::os::unix::net::UnixStream::set_write_timeout(self, Some(timeout))
    }
}

#[cfg(windows)]
impl DeadlineStream for tgskk_process::Pipe {
    fn arm_read(&self, timeout: Duration) -> io::Result<()> {
        self.set_read_timeout(Some(timeout))
    }
    fn arm_write(&self, timeout: Duration) -> io::Result<()> {
        self.set_write_timeout(Some(timeout))
    }
}

/// Writes one request and reads one bounded, validated response frame. The
/// deadline is rechecked after the frame is read, uniformly for fresh TCP
/// connections and persistent helper sockets.
pub(crate) fn exchange<S: DeadlineStream>(
    stream: &mut S,
    request: &[u8],
    encoding: Encoding,
    deadline: Instant,
    trailing: TrailingData,
) -> Result<Vec<Candidate>, ClientError> {
    write_request(stream, request, deadline)?;
    let frame = read_frame(stream, deadline, trailing)?;
    if Instant::now() >= deadline {
        return Err(ClientError::Timeout);
    }
    decode_response(&frame, encoding)
        .map_err(|err| ClientError::Protocol(ProtocolError::Codec(err)))
}

/// Minimum socket timeout. Durations below the platform timer resolution can
/// round to zero, which means "no timeout" rather than "expire immediately".
const MIN_SOCKET_TIMEOUT: Duration = Duration::from_millis(1);

fn remaining(deadline: Instant) -> Result<Duration, ClientError> {
    let now = Instant::now();
    if now >= deadline {
        return Err(ClientError::Timeout);
    }
    Ok((deadline - now).max(MIN_SOCKET_TIMEOUT))
}

fn map_io(err: io::Error) -> ClientError {
    match err.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => ClientError::Timeout,
        _ => ClientError::Io(err),
    }
}

fn write_request<S: DeadlineStream>(
    stream: &mut S,
    request: &[u8],
    deadline: Instant,
) -> Result<(), ClientError> {
    let mut written = 0;
    while written < request.len() {
        let timeout = remaining(deadline)?;
        stream.arm_write(timeout).map_err(ClientError::Io)?;
        match stream.write(&request[written..]) {
            Ok(0) => return Err(ClientError::Io(io::Error::from(io::ErrorKind::WriteZero))),
            Ok(count) => written += count,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(err) => return Err(map_io(err)),
        }
    }
    Ok(())
}

fn read_frame<S: DeadlineStream>(
    stream: &mut S,
    deadline: Instant,
    trailing: TrailingData,
) -> Result<Vec<u8>, ClientError> {
    let mut frame = Vec::new();
    let mut buffer = [0u8; 1024];

    loop {
        let timeout = remaining(deadline)?;
        stream.arm_read(timeout).map_err(ClientError::Io)?;
        let count = match stream.read(&mut buffer) {
            Ok(0) => return Err(ClientError::Protocol(ProtocolError::Truncated)),
            Ok(count) => count,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(map_io(err)),
        };

        if frame.is_empty() && buffer[0] == b'0' {
            return Err(ClientError::Protocol(ProtocolError::ServerError));
        }

        let chunk = &buffer[..count];
        if let Some(position) = chunk.iter().position(|&byte| byte == b'\n') {
            let needed = frame.len() + position + 1;
            if needed > MAX_RESPONSE_BYTES {
                return Err(ClientError::Protocol(ProtocolError::Oversized));
            }
            if trailing == TrailingData::Reject && position + 1 != chunk.len() {
                return Err(ClientError::Protocol(ProtocolError::TrailingData));
            }
            frame.extend_from_slice(&chunk[..=position]);
            return Ok(frame);
        }

        if frame.len() + count > MAX_RESPONSE_BYTES {
            return Err(ClientError::Protocol(ProtocolError::Oversized));
        }
        frame.extend_from_slice(chunk);
    }
}
