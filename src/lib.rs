/// New type to cancel synchronous reads and writes.
///
/// This crate provides a _new-type_ [Cancellable] that can be used to wrap
/// a `Read`, `Write` or `Seek`, so that its operation can be interrupted at
/// any time.
///
/// To signal the cancellation event, you first create a [CancellationToken],
/// and then call its `CancellationToken::cancel` member function.
///
/// You can use the same `CancellationToken for as many `Cancellable` objects
/// as you need.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// This type signals a cancellation event.
///
/// It is `Sync` and `Send` so you can share it between threads freely.
///
/// It also implements `Eq`, `Ord` and `Hash`, with some arbitrary ordering,
/// so that you can use it as a cheap identifier for your interruptible actions.
/// All clones of the same token will compare equal.
#[derive(Clone, Default, Debug)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl PartialEq for CancellationToken {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cancelled, &other.cancelled)
    }
}

impl Eq for CancellationToken {}

impl Ord for CancellationToken {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cancelled.as_ptr().cmp(&other.cancelled.as_ptr())
    }
}

impl PartialOrd for CancellationToken {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for CancellationToken {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cancelled.as_ptr().hash(state);
    }
}

impl CancellationToken {
    /// Creates a new `CancellationToken`, in a non-cancelled state.
    pub fn new() -> Self {
        Self::default()
    }
    /// Signals this token as _cancelled_.
    ///
    /// Note that it takes a non-mutable `self`, so you are able to cancel a
    /// shared token.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    /// Checks whether a token is cancelled.
    ///
    /// It returns `Ok(())` if non-cancelled, `Err(ErrorKind::BrokenPipe)` if cancelled.
    pub fn check(&self) -> std::io::Result<()> {
        let cancelled = self.cancelled.load(Ordering::Relaxed);
        if cancelled {
            Err(std::io::ErrorKind::BrokenPipe.into())
        } else {
            Ok(())
        }
    }
}

/// A newtype around `CancellationToken` that automatically cancels on `drop`.
pub struct CancellationGuard(pub CancellationToken);

impl Drop for CancellationGuard {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

/// A newtype around any `Read`, `Write` or `Seek` value, that makes it cancellable.
pub struct Cancellable<T> {
    inner: T,
    token: CancellationToken,
}

impl<T> Cancellable<T> {
    /// Wraps a value as `Cancellable`.
    pub fn new(inner: T, token: CancellationToken) -> Self {
        Self { inner, token }
    }
    /// Gets the inner token.
    ///
    /// You will probably need to clone it if you want store it somewhere.
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }
    /// Unwraps the inner value.
    pub fn into_inner(self) -> T {
        self.inner
    }
    /// Gets a reference to the inner value.
    pub fn get_ref(&self) -> &T {
        &self.inner
    }
    /// Gets a mutable reference to the inner value.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: std::io::Read> std::io::Read for Cancellable<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.token.check()?;
        self.inner.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> std::io::Result<usize> {
        self.token.check()?;
        self.inner.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.token.check()?;
        self.inner.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.token.check()?;
        self.inner.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.token.check()?;
        self.inner.read_exact(buf)
    }
}

impl<T: std::io::Write> std::io::Write for Cancellable<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.token.check()?;
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.token.check()?;
        self.inner.flush()
    }
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        self.token.check()?;
        self.inner.write_vectored(bufs)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.token.check()?;
        self.inner.write_all(buf)
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        self.token.check()?;
        self.inner.write_fmt(fmt)
    }
}

impl<T: std::io::Seek> std::io::Seek for Cancellable<T> {
    fn seek(&mut self, from: std::io::SeekFrom) -> std::io::Result<u64> {
        self.token.check()?;
        self.inner.seek(from)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        self.token.check()?;
        self.inner.rewind()
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        self.token.check()?;
        self.inner.stream_position()
    }

    fn seek_relative(&mut self, offset: i64) -> std::io::Result<()> {
        self.token.check()?;
        self.inner.seek_relative(offset)
    }
}

impl<T: std::io::BufRead> std::io::BufRead for Cancellable<T> {
    // Provided methods are not wrapped, probably not worth it
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.token.check()?;
        self.inner.fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::{self, Read, Seek, Write};
    use std::time::Duration;

    fn inf_write(ct: CancellationToken) -> io::Result<()> {
        let w = io::empty();
        let mut w = Cancellable::new(w, ct);
        for _i in 0..10 {
            w.write_all(&[0])?;
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    fn inf_read(ct: CancellationToken) -> io::Result<()> {
        let r = io::empty();
        let mut r = Cancellable::new(r, ct);
        let mut data = [0];
        for _i in 0..10 {
            r.read(&mut data)?;
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    fn inf_seek(ct: CancellationToken) -> io::Result<()> {
        let r = io::empty();
        let mut r = Cancellable::new(r, ct);
        for _i in 0..10 {
            r.seek(io::SeekFrom::Start(0))?;
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    #[test]
    fn test_write() {
        let ct = CancellationToken::new();
        let th = std::thread::spawn({
            let ct = ct.clone();
            move || {
                inf_write(ct).unwrap();
            }
        });
        ct.cancel();
        let err = th.join().unwrap_err();
        let err = err.downcast::<String>().unwrap();
        assert!(err.contains("BrokenPipe"));
    }

    #[test]
    fn test_guard() {
        let th;
        {
            let cg = CancellationGuard(CancellationToken::new());
            th = std::thread::spawn({
                let ct = cg.0.clone();
                move || {
                    inf_write(ct).unwrap();
                }
            });
        }
        let err = th.join().unwrap_err();
        let err = err.downcast::<String>().unwrap();
        assert!(err.contains("BrokenPipe"));
    }

    #[test]
    fn test_read() {
        let ct = CancellationToken::new();
        let th = std::thread::spawn({
            let ct = ct.clone();
            move || {
                inf_read(ct).unwrap();
            }
        });
        ct.cancel();
        let err = th.join().unwrap_err();
        let err = err.downcast::<String>().unwrap();
        assert!(err.contains("BrokenPipe"));
    }

    #[test]
    fn test_seek() {
        let ct = CancellationToken::new();
        let th = std::thread::spawn({
            let ct = ct.clone();
            move || {
                inf_seek(ct).unwrap();
            }
        });
        ct.cancel();
        let err = th.join().unwrap_err();
        let err = err.downcast::<String>().unwrap();
        assert!(err.contains("BrokenPipe"));
    }
}
