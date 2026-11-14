//! Managed helper process channels. The caller owns the child and must kill and
//! reap it before dropping its stream. Unix uses a socketpair; Windows uses
//! bounded stdio reader/writer threads with deadlines.
//!
//! Windows spawning does not filter other inheritable handles in the parent.
//! Use a dedicated worker/broker process, rather than an arbitrary application
//! host, until explicit handle-list isolation is available.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::{Stream, spawn};
#[cfg(windows)]
pub use windows::{Pipe, spawn};
#[cfg(windows)]
pub type Stream = Pipe;
