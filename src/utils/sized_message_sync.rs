use std::io::{Read, Write};

use serde::de::DeserializeOwned;

use crate::{
    sherlock_msg,
    tokio_utils::SizedMessageObj,
    utils::errors::{
        ResultExt,
        SherlockMessage,
        types::{ErrorKind, SocketAction},
    },
};

#[allow(dead_code)]
pub trait SizedMessage {
    /// Writes a message to the channel with a length prefix.
    ///
    /// The message length is encoded as a 4-byte big-endian `u32` before
    /// the message itself. This allows the receiver to know exactly how
    /// many bytes to read.
    ///
    /// # Errors
    ///
    /// Returns an `std::io::Error` if:
    /// - Writing to the underlying channel fails, or
    /// - The message is too large to fit in a `u32` (greater than 4 GiB).
    fn write_sized(&mut self, what: SizedMessageObj) -> Result<(), SherlockMessage>;

    /// Reads a length-prefixed message from the channel.
    ///
    /// Expects the first 4 bytes to be a big-endian `u32` representing
    /// the length of the message, followed by exactly that many bytes.
    ///
    /// # Errors
    ///
    /// Returns an `std::io::Error` if:
    /// - Reading from the underlying channel fails, or
    /// - The indicated message length is unreasonably large or invalid.
    fn read_sized<T: DeserializeOwned>(&mut self) -> Result<T, SherlockMessage>;
}
impl<S: Read + Write> SizedMessage for S {
    fn write_sized(&mut self, what: SizedMessageObj) -> Result<(), SherlockMessage> {
        // Safely convert buf_len from usize to u32
        let buf_len: u32 = what
            .bytes()
            .len()
            .try_into()
            .or_kind(ErrorKind::InvalidData)?;

        // Write message size to stream
        let len_bytes = buf_len.to_be_bytes();
        self.write_all(&len_bytes)
            .or_kind_with(|| ErrorKind::Socket(SocketAction::Write))?;

        // Write message to stream
        self.write_all(what.bytes())
            .or_kind_with(|| ErrorKind::Socket(SocketAction::Write))?;

        Ok(())
    }
    fn read_sized<T: DeserializeOwned>(&mut self) -> Result<T, SherlockMessage> {
        let mut buf_len = [0u8; 4];

        // Read message length
        self.read_exact(&mut buf_len)
            .or_kind_with(|| ErrorKind::Socket(SocketAction::Read))?;
        let msg_len = u32::from_be_bytes(buf_len) as usize;

        if msg_len > 1024 * 1024 {
            return Err(sherlock_msg!(
                Warning,
                ErrorKind::Io,
                "Invalid message size received"
            ));
        }

        let mut buf = vec![0u8; msg_len];
        self.read_exact(&mut buf)
            .or_kind_with(|| ErrorKind::Socket(SocketAction::Read))?;

        let cfg = bincode::config::standard();
        bincode::serde::decode_from_slice::<T, _>(&buf, cfg)
            .map(|(val, _)| val)
            .map_err(|e| {
                sherlock_msg!(
                    Warning,
                    ErrorKind::Parse("Socket Message".into()),
                    e.to_string()
                )
            })
    }
}
