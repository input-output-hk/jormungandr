use crate::ByteArray;
use std::marker::PhantomData;

/// A dynamically created buffer for T
#[derive(Clone)]
pub struct ByteBuilder<T> {
    buffer: Vec<u8>,
    phantom: PhantomData<T>,
}

impl<T> ByteBuilder<T> {
    /// Create an unconstrained Builder
    pub fn new() -> Self {
        ByteBuilder {
            buffer: Vec::new(),
            phantom: PhantomData,
        }
    }

    /// Create a builder of fixed size
    pub fn new_fixed(size: usize) -> Self {
        ByteBuilder {
            buffer: Vec::with_capacity(size),
            phantom: PhantomData,
        }
    }

    /// Append an u8 in the builder
    pub fn u8(self, v: u8) -> Self {
        let mut buf = self.buffer;
        buf.push(v);
        ByteBuilder {
            buffer: buf,
            phantom: self.phantom,
        }
    }
    /// Append bytes in the builder
    pub fn bytes(self, v: &[u8]) -> Self {
        let mut buf = self.buffer;
        buf.extend_from_slice(v);
        ByteBuilder {
            buffer: buf,
            phantom: self.phantom,
        }
    }

    /// write an iterator of maximum 256 items using the closure F
    ///
    /// note that the buffer contains a byte to represent the size
    /// of the list
    pub fn iter8<F, I>(self, mut l: I, f: F) -> Self
      where I: Iterator + ExactSizeIterator,
            F: Fn(Self, &I::Item) -> Self,
    {
        assert!(l.len() < 256);
        let mut bb = self.u8(l.len() as u8);
        while let Some(ref i) = l.next() {
            bb = f(bb, i)
        }
        bb
    }

    /// Append an u16 in the builder
    pub fn u16(self, v: u16) -> Self {
        self.bytes(&v.to_be_bytes())
    }

    /// Append an u32 in the builder
    pub fn u32(self, v: u32) -> Self {
        self.bytes(&v.to_be_bytes())
    }

    /// Append an u64 in the builder
    pub fn u64(self, v: u64) -> Self {
        self.bytes(&v.to_be_bytes())
    }

    /// Append an u128 in the builder
    pub fn u128(self, v: u128) -> Self {
        self.bytes(&v.to_be_bytes())
    }

    /// Finalize the buffer and return a fixed ByteArray of T
    pub fn finalize(self) -> ByteArray<T> {
        ByteArray::from_vec(self.buffer)
    }
}
