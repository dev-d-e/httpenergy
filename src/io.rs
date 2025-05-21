use std::io::Error;

///A trait for reading bytes from a source.
pub trait ReadByte {
    ///Returns the number of bytes between the current position and the end.
    fn surplus(&self) -> usize;

    ///Advance the internal cursor.
    fn advance(&mut self, n: usize);

    ///Gets an unsigned 8 bit integer from self.
    fn fetch(&mut self) -> Option<u8>;

    ///Gets at most n bytes from self.
    fn fetch_all(&mut self, n: usize) -> Option<&[u8]>;

    ///Gets some bytes from self.
    #[inline]
    fn fetch_some(&mut self) -> Option<&[u8]> {
        let n = self.surplus();
        self.fetch_all(n)
    }

    ///Gets an unsigned 16 bit integer from self in big-endian byte order.
    #[inline]
    fn fetch_u16(&mut self) -> Option<u16> {
        if let Some(v) = self.fetch_all(2) {
            if v.len() == 2 {
                return Some(u16::from_be_bytes([v[0], v[1]]));
            }
        }
        None
    }

    ///Gets an unsigned 32 bit integer from self in the big-endian byte order.
    #[inline]
    fn fetch_u32(&mut self) -> Option<u32> {
        if let Some(v) = self.fetch_all(4) {
            if v.len() == 4 {
                return Some(u32::from_be_bytes([v[0], v[1], v[2], v[3]]));
            }
        }
        None
    }

    ///Gets an unsigned 64 bit integer from self in big-endian byte order.
    #[inline]
    fn fetch_u64(&mut self) -> Option<u64> {
        if let Some(v) = self.fetch_all(8) {
            if v.len() == 8 {
                return Some(u64::from_be_bytes([
                    v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7],
                ]));
            }
        }
        None
    }

    ///Gets an unsigned 128 bit integer from self in big-endian byte order.
    #[inline]
    fn fetch_u128(&mut self) -> Option<u128> {
        if let Some(v) = self.fetch_all(16) {
            if v.len() == 8 {
                return Some(u128::from_be_bytes([
                    v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8], v[9], v[10], v[11],
                    v[12], v[13], v[14], v[15],
                ]));
            }
        }
        None
    }

    ///Returns true if there are any more bytes to read.
    #[inline]
    fn has_surplus(&self) -> bool {
        self.surplus() > 0
    }
}

impl ReadByte for &[u8] {
    #[inline]
    fn surplus(&self) -> usize {
        self.len()
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        if n < self.len() {
            *self = &self[n..];
        } else {
            *self = &[];
        }
    }

    #[inline]
    fn fetch(&mut self) -> Option<u8> {
        if self.len() > 0 {
            let r = self[0];
            self.advance(1);
            Some(r)
        } else {
            None
        }
    }

    #[inline]
    fn fetch_all(&mut self, n: usize) -> Option<&[u8]> {
        if n <= self.len() {
            let r = &self[..n];
            self.advance(n);
            Some(r)
        } else {
            None
        }
    }
}

///A trait for writing bytes to a buffer.
pub trait WriteByte {
    ///Returns the number of bytes that can be written from the current position until the end.
    fn surplus_mut(&self) -> usize;

    ///Writes an unsigned 8 bit integer to self.
    fn put(&mut self, o: u8) -> Option<Error>;

    ///Writes repetitive cnt bytes an unsigned 8 bit integer to self.
    fn put_repeat(&mut self, cnt: usize, o: u8) -> Option<Error>;

    ///Writes a buffer to self.
    ///self must have enough surplus to contain all bytes.
    fn put_all(&mut self, buf: &[u8]) -> Option<Error>;

    ///Writes a buffer to self, returning the bytes which were not written.
    #[inline]
    fn put_some(&mut self, buf: &[u8]) -> Result<Option<&[u8]>, Error> {
        if let Some(e) = self.put_all(buf) {
            Err(e)
        } else {
            Ok(None)
        }
    }

    ///Writes an unsigned 16 bit integer to self in big-endian byte order.
    #[inline]
    fn put_u16(&mut self, o: u16) -> Option<Error> {
        self.put_all(&o.to_be_bytes())
    }

    ///Writes an unsigned 32 bit integer to self in big-endian byte order.
    #[inline]
    fn put_u32(&mut self, o: u32) -> Option<Error> {
        self.put_all(&o.to_be_bytes())
    }

    ///Writes an unsigned 64 bit integer to self in the big-endian byte order.
    #[inline]
    fn put_u64(&mut self, o: u64) -> Option<Error> {
        self.put_all(&o.to_be_bytes())
    }

    ///Writes an unsigned 128 bit integer to self in the big-endian byte order.
    #[inline]
    fn put_u128(&mut self, o: u128) -> Option<Error> {
        self.put_all(&o.to_be_bytes())
    }

    ///Returns true if there is space in self for more bytes.
    #[inline]
    fn has_surplus_mut(&self) -> bool {
        self.surplus_mut() > 0
    }
}

impl WriteByte for Vec<u8> {
    #[inline]
    fn surplus_mut(&self) -> usize {
        isize::MAX as usize - self.len()
    }

    #[inline]
    fn put(&mut self, o: u8) -> Option<Error> {
        self.push(o);
        None
    }

    fn put_repeat(&mut self, cnt: usize, o: u8) -> Option<Error> {
        self.resize(self.len() + cnt, o);
        None
    }

    #[inline]
    fn put_some(&mut self, buf: &[u8]) -> Result<Option<&[u8]>, Error> {
        self.extend_from_slice(buf);
        Ok(None)
    }

    #[inline]
    fn put_all(&mut self, buf: &[u8]) -> Option<Error> {
        self.extend_from_slice(buf);
        None
    }
}

///Wraps a WriteByte and buffers its output.
pub struct BufWriteByte<T>
where
    T: WriteByte,
{
    buf: Vec<u8>,
    buf_size: usize,
    inner: T,
}

impl<T> BufWriteByte<T>
where
    T: WriteByte,
{
    ///Creates a new Self with the specified buffer capacity.
    pub fn new(inner: T, buf_size: usize) -> Self {
        Self {
            buf: Vec::with_capacity(buf_size),
            buf_size,
            inner,
        }
    }

    ///Creates a new Self with a default buffer capacity. The default is currently 4096.
    pub fn with_buffer(inner: T) -> Self {
        Self::new(inner, 4096)
    }

    #[inline]
    fn put_check(&mut self) -> Option<Error> {
        if self.buf.len() >= self.buf_size {
            self.inner.put_all(&self.buf)?;
            self.buf.clear();
        }
        None
    }
}

impl<T> WriteByte for BufWriteByte<T>
where
    T: WriteByte,
{
    #[inline]
    fn surplus_mut(&self) -> usize {
        self.buf.surplus_mut()
    }

    #[inline]
    fn put(&mut self, o: u8) -> Option<Error> {
        self.put_check()?;
        self.buf.put(o)
    }

    #[inline]
    fn put_repeat(&mut self, cnt: usize, o: u8) -> Option<Error> {
        self.put_check()?;
        if self.buf.len() + cnt >= self.buf_size {
            self.inner.put_all(&self.buf)?;
            self.buf.clear();
            self.inner.put_repeat(cnt, o)
        } else {
            self.buf.put_repeat(cnt, o)
        }
    }

    #[inline]
    fn put_all(&mut self, o: &[u8]) -> Option<Error> {
        self.put_check()?;
        if self.buf.len() + o.len() >= self.buf_size {
            self.inner.put_all(&self.buf)?;
            self.buf.clear();
            self.inner.put_all(o)
        } else {
            self.buf.put_all(o)
        }
    }
}
