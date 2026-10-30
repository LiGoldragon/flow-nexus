//! Linux operating-system primitives used by Flow Nexus adapters.
#![forbid(unsafe_op_in_unsafe_fn)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BytesRead(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Errno(pub i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntropyError {
    Interrupted,
    Unavailable,
    OperatingSystem(Errno),
    InvalidReturn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GetrandomFlags(u32);

impl Default for GetrandomFlags {
    fn default() -> Self {
        Self(libc::GRND_NONBLOCK)
    }
}

pub struct LinuxEntropy {
    flags: GetrandomFlags,
}

impl Default for LinuxEntropy {
    fn default() -> Self {
        Self {
            flags: GetrandomFlags::default(),
        }
    }
}

pub trait OsEntropyReadable {
    type Error;
    type Read;
    fn read_entropy(&mut self, buffer: &mut [u8]) -> Result<Self::Read, Self::Error>;
}

impl OsEntropyReadable for LinuxEntropy {
    type Error = EntropyError;
    type Read = BytesRead;

    fn read_entropy(&mut self, buffer: &mut [u8]) -> Result<BytesRead, EntropyError> {
        if buffer.is_empty() {
            return Ok(BytesRead(0));
        }
        // SAFETY: `buffer` is the caller-owned initialized, writable slice for the duration
        // of this call. Its pointer and bounded length are passed only to the syscall; no
        // pointer is retained or aliased after the call returns.
        let result =
            unsafe { libc::getrandom(buffer.as_mut_ptr().cast(), buffer.len(), self.flags.0) };
        if result < 0 {
            // Capture thread-local errno before any intervening OS operation.
            let errno = std::io::Error::last_os_error()
                .raw_os_error()
                .ok_or(EntropyError::InvalidReturn)?;
            return Err(match errno {
                libc::EINTR => EntropyError::Interrupted,
                libc::EAGAIN => EntropyError::Unavailable,
                value => EntropyError::OperatingSystem(Errno(value)),
            });
        }
        let count = usize::try_from(result).map_err(|_| EntropyError::InvalidReturn)?;
        if count > buffer.len() {
            return Err(EntropyError::InvalidReturn);
        }
        Ok(BytesRead(count))
    }
}
