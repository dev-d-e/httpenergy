use derive_more::{Debug, Deref, DerefMut};
use std::num::NonZeroUsize;

pub(crate) const READ_BYTE_ERROR: &str = "read byte error";

///A trait for reading bytes.
pub trait GetU8 {
    ///Returns the number of bytes between the current position and the end.
    fn surplus(&self) -> usize;

    ///Returns the internal index.
    fn index(&mut self) -> usize;

    ///Sets the internal index.
    fn set_index(&mut self, n: usize);

    ///Returns an unsigned 8 bit integer.
    fn get_u8(&mut self) -> Option<u8>;

    ///Returns n bytes.
    fn get_exact(&mut self, n: usize) -> Option<&[u8]>;

    ///Returns n bytes.
    fn get_exact_to<'a>(&'a mut self, n: usize) -> Option<Box<dyn GetU8 + 'a>>;

    ///Splits n bytes.
    fn split_exact(&mut self, n: usize) -> Option<Vec<u8>>;

    ///Returns surplus bytes.
    fn get_surplus<'a>(&'a mut self) -> &'a [u8];

    ///Returns a sub `GetU8`, or None if out of bounds.
    fn sub_to<'a>(&'a self, a: usize, b: usize) -> Option<Box<dyn GetU8 + 'a>>;

    ///Returns true if there are any more bytes to read.
    #[inline]
    fn is_surplus(&self) -> bool {
        self.surplus() > 0
    }
}

///A `GetU8` over the bytes of a slice.
#[derive(Debug, Deref)]
pub struct SliceGet<'a> {
    #[debug("{}", o.len())]
    #[deref]
    o: &'a [u8],
    i: usize,
}

impl<'a> From<&'a [u8]> for SliceGet<'a> {
    fn from(o: &'a [u8]) -> Self {
        Self { o, i: 0 }
    }
}

impl<'a> GetU8 for SliceGet<'a> {
    #[inline]
    fn surplus(&self) -> usize {
        self.o.len().saturating_sub(self.i)
    }

    #[inline]
    fn index(&mut self) -> usize {
        self.i
    }

    #[inline]
    fn set_index(&mut self, n: usize) {
        self.i = n;
    }

    #[inline]
    fn get_u8(&mut self) -> Option<u8> {
        let a = self.i;
        if a < self.o.len() {
            self.i += 1;
            self.o.get(a).copied()
        } else {
            None
        }
    }

    #[inline]
    fn get_exact(&mut self, n: usize) -> Option<&'a [u8]> {
        let a = self.i;
        let b = a + n;
        if b <= self.o.len() {
            self.i = b;
            self.o.get(a..b)
        } else {
            None
        }
    }

    #[inline]
    fn get_exact_to(&mut self, n: usize) -> Option<Box<dyn GetU8 + 'a>> {
        let a = self.i;
        let b = a + n;
        if b <= self.o.len() {
            self.i = b;
            self.o
                .get(a..b)
                .map(|o| Box::new(o.into_get()) as Box<dyn GetU8>)
        } else {
            None
        }
    }

    #[inline]
    fn split_exact(&mut self, n: usize) -> Option<Vec<u8>> {
        self.get_exact(n).map(|o| o.to_vec())
    }

    #[inline]
    fn get_surplus(&mut self) -> &[u8] {
        let a = self.i;
        self.i = self.o.len();
        self.o.get(a..).unwrap_or_else(|| &[])
    }

    #[inline]
    fn sub_to(&self, a: usize, b: usize) -> Option<Box<dyn GetU8 + 'a>> {
        self.o
            .get(a..b)
            .map(|o| Box::new(o.into_get()) as Box<dyn GetU8>)
    }
}

///A `GetU8` over the bytes of a vector.
#[derive(Debug, Deref, DerefMut)]
pub struct VecGet {
    #[debug("{}", o.len())]
    #[deref]
    #[deref_mut]
    o: Vec<u8>,
    i: usize,
}

impl<T: Into<Vec<u8>>> From<T> for VecGet {
    fn from(o: T) -> Self {
        Self { o: o.into(), i: 0 }
    }
}

impl VecGet {
    pub fn take(self) -> Vec<u8> {
        self.o
    }
}

impl GetU8 for VecGet {
    #[inline]
    fn surplus(&self) -> usize {
        self.o.len().saturating_sub(self.i)
    }

    #[inline]
    fn index(&mut self) -> usize {
        self.i
    }

    #[inline]
    fn set_index(&mut self, n: usize) {
        self.i = n;
    }

    #[inline]
    fn get_u8(&mut self) -> Option<u8> {
        let a = self.i;
        if a < self.o.len() {
            self.i += 1;
            self.o.get(a).copied()
        } else {
            None
        }
    }

    #[inline]
    fn get_exact(&mut self, n: usize) -> Option<&[u8]> {
        let a = self.i;
        let b = a + n;
        if b <= self.o.len() {
            self.i = b;
            self.o.get(a..b)
        } else {
            None
        }
    }

    #[inline]
    fn get_exact_to<'a>(&'a mut self, n: usize) -> Option<Box<dyn GetU8 + 'a>> {
        let a = self.i;
        let b = a + n;
        if b <= self.o.len() {
            self.i = b;
            self.o
                .get(a..b)
                .map(|o| Box::new(o.into_get()) as Box<dyn GetU8>)
        } else {
            None
        }
    }

    #[inline]
    fn split_exact(&mut self, n: usize) -> Option<Vec<u8>> {
        let a = self.i;
        let b = a + n;
        if b <= self.o.len() {
            let mut r = self.o.split_off(a);
            self.o = r.split_off(n);
            self.i = 0;
            return Some(r);
        }
        None
    }

    #[inline]
    fn get_surplus(&mut self) -> &[u8] {
        let a = self.i;
        self.i = self.o.len();
        self.o.get(a..).unwrap_or_else(|| &[])
    }

    #[inline]
    fn sub_to<'a>(&'a self, a: usize, b: usize) -> Option<Box<dyn GetU8 + 'a>> {
        self.o
            .get(a..b)
            .map(|o| Box::new(o.into_get()) as Box<dyn GetU8>)
    }
}

impl PutU8 for VecGet {
    #[inline]
    fn blank(&self) -> usize {
        isize::MAX as usize - self.len()
    }

    #[inline]
    fn put_u8(&mut self, o: u8) -> bool {
        self.push(o);
        true
    }

    #[inline]
    fn put_repeat(&mut self, n: usize, o: u8) -> bool {
        self.o.resize(self.o.len() + n, o);
        true
    }

    #[inline]
    fn put_exact(&mut self, o: &[u8]) -> bool {
        self.extend_from_slice(o);
        true
    }
}

///A trait for conversion into [`GetU8`].
pub trait IntoGetU8 {
    type Item: GetU8;

    fn into_get(self) -> Self::Item;
}

impl<'a> IntoGetU8 for &'a [u8] {
    type Item = SliceGet<'a>;

    fn into_get(self) -> Self::Item {
        self.into()
    }
}

impl IntoGetU8 for Vec<u8> {
    type Item = VecGet;

    fn into_get(self) -> Self::Item {
        self.into()
    }
}

///A trait for writing bytes.
pub trait PutU8 {
    ///Returns the number of bytes that can be written.
    fn blank(&self) -> usize;

    ///Returns true if there is space in self for more bytes.
    #[inline]
    fn is_blank(&self) -> bool {
        self.blank() > 0
    }

    ///Writes an unsigned 8 bit integer.
    fn put_u8(&mut self, o: u8) -> bool;

    ///Writes bytes, self must have enough blank to contain all bytes.
    fn put_exact(&mut self, o: &[u8]) -> bool;

    ///Writes repetitive n bytes.
    fn put_repeat(&mut self, n: usize, o: u8) -> bool;
}

impl PutU8 for Vec<u8> {
    #[inline]
    fn blank(&self) -> usize {
        isize::MAX as usize - self.len()
    }

    #[inline]
    fn put_u8(&mut self, o: u8) -> bool {
        self.push(o);
        true
    }
    #[inline]
    fn put_exact(&mut self, o: &[u8]) -> bool {
        self.extend_from_slice(o);
        true
    }

    #[inline]
    fn put_repeat(&mut self, n: usize, o: u8) -> bool {
        self.resize(self.len() + n, o);
        true
    }
}

///A vector with a fixed capacity.
#[derive(Debug, Deref)]
pub struct FiniteVec {
    capacity: NonZeroUsize,
    #[debug("{}", inner.len())]
    #[deref]
    inner: Vec<u8>,
}
impl FiniteVec {
    ///Returns capacity.
    pub fn capacity(&self) -> usize {
        self.capacity.get()
    }
}

impl From<NonZeroUsize> for FiniteVec {
    ///Creates a new vector with capacity.
    fn from(capacity: NonZeroUsize) -> Self {
        Self {
            capacity,
            inner: Vec::with_capacity(capacity.get()),
        }
    }
}

impl From<(NonZeroUsize, Vec<u8>)> for FiniteVec {
    fn from(o: (NonZeroUsize, Vec<u8>)) -> Self {
        Self {
            capacity: o.0,
            inner: o.1,
        }
    }
}

impl PutU8 for FiniteVec {
    #[inline]
    fn blank(&self) -> usize {
        self.capacity().saturating_sub(self.len())
    }

    #[inline]
    fn put_u8(&mut self, o: u8) -> bool {
        if self.is_blank() {
            self.inner.push(o);
            true
        } else {
            false
        }
    }

    #[inline]
    fn put_exact(&mut self, o: &[u8]) -> bool {
        if self.blank() >= o.len() {
            self.inner.extend_from_slice(o);
            true
        } else {
            false
        }
    }

    #[inline]
    fn put_repeat(&mut self, n: usize, o: u8) -> bool {
        if self.blank() >= n {
            self.inner.resize(self.inner.len() + n, o);
            true
        } else {
            false
        }
    }
}
