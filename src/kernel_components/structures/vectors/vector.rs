/// A vector implementation.
/// 
use core::ptr;
use core::mem;
use core::marker::PhantomData;
use core::alloc::{Layout, GlobalAlloc};
use core::ops::{Deref, DerefMut};

use crate::AsBytes;
use super::raw_vector::RawVec;

/// Fully working vector implementation that allocates on the global heap..
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Vec<T: Sized> {
    pub buf: RawVec<T>,
    /// The amount of elements stored in the vector.
    pub len: usize,
    /// Phantom data is needed to convince the compiler that vector do own T.
    _own_t: PhantomData<T>,
}

impl<T> Vec<T> {
    /// Creates a new instance of empty vector.
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            buf: RawVec::new(),
            len: 0,
            _own_t: PhantomData::default(),
        }
    }

    #[inline]
    pub fn from_array(array: &[T]) -> Self where T: Copy {
        let mut temp_vec = Self::new();

        array
            .into_iter()
            .for_each(|item| temp_vec.push(*item));

        temp_vec
    }

    #[inline]
    pub fn from(item: T) -> Self {
        let mut temp_vec = Self::new();
        temp_vec.push(item);
        temp_vec
    }

    /// Extends the vector by Iterator.
    #[inline(always)]
    pub fn extend<I: Iterator<Item = T>>(&mut self, iter: I) {
        iter.map(|item| self.push(item));
    }

    /// Pushes a value into the vectors head.
    #[inline(always)]
    pub fn push(&mut self, element: T) {
        if self.len == self.cap() { self.buf.grow() }

        unsafe {
            ptr::write(self.ptr().add(self.len), element);
        }

        self.len += 1;
    }

    /// Pops the last element of the vector and returns it's value.
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe {
                Some(ptr::read(self.ptr().add(self.len)))
            }
        }
    }

    /// Inserts element at given index and shifts the rest to right by one.
    /// 
    /// # Panics
    /// 
    /// Panic will occur if the index is out of bounds
    #[inline(always)]
    pub fn insert(&mut self, index: usize, element: T) {
        assert!(index <= self.len, "Index out of bounds. Current vector's length is {}. Got index: {}. Make sure the index is <= than the length of the vector.", self.len, index);
        if self.cap() == self.len { self.buf.grow() }

        unsafe {
            ptr::copy(
                self.ptr().add(index), 
                self.ptr().add(index + 1), 
                self.len - index,
            );
            ptr::write(self.ptr().add(index), element);
            self.len += 1;
        }
    }

    /// Removes the element from the given index and shifts the rest to left by one.
    /// 
    /// # Panics
    /// 
    /// Panic will occur if the index is out of bounds
    #[inline(always)]
    pub fn remove(&mut self, index: usize) -> Option<T> {
        assert!(index < self.len, "Index out of bounds. Current vector's length is {}. Got index: {}. Make sure the index is < than the length of the vector.", self.len, index);
        
        self.len -= 1;
        unsafe {
            let result = Some(ptr::read(self.ptr().add(index)));
            ptr::copy(
                self.ptr().add(index + 1), 
                self.ptr().add(index), 
                self.len - index,
            );
            result
            }
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self
    }

    fn ptr(&self) -> *mut T {
        self.buf.ptr.as_ptr()
    }

    fn cap(&self) -> usize {
        self.buf.cap
    }

}

/// The vector is save to transfer via threads
unsafe impl<T: Send> Send for Vec<T> {}
unsafe impl<T: Sync> Sync for Vec<T> {}

/// Drop for vector to not leak data.
impl<T: Sized> Drop for Vec<T> {
    fn drop(&mut self) {
        if self.cap() != 0 {
            while let Some(_) = self.pop() { /* Deleting all the elements */ }
        }
    }
}

/// Deref trait for vector
/// 
/// Since the vector is an array, we would want to receive a slice of elements from it.
impl<T> Deref for Vec<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe {
            core::slice::from_raw_parts(self.ptr(), self.len)
        }
    }
}

/// DerefMut trait for vector
/// 
/// Looks works in pretty much the same way
impl<T> DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe {
            core::slice::from_raw_parts_mut(self.ptr(), self.len)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vector does not allow ZST's, so it should panic on empty data types.
    #[test_case]
    #[should_panic]
    fn vector_with_zst() {
        enum Empty {}

        let _vector: Vec<Empty> = Vec::new();
    }

    // Operations on vector
    #[test_case]
    fn vec_ops() {
        let mut vector: Vec<u8> = Vec::new();

        for i in 0..50 {
            vector.push(i);
        }

        for i in (0..vector.len()).filter(|num| num % 2 == 0) {
            vector.remove(i);
            vector.insert(i / 2, 0);
        }

        for _ in 0..vector.len() {
            vector.pop();
        }

        assert_eq!(0, vector.len());
    }

    // Checks for proper deallocating the memory. No memory leaks!
    #[test_case]
    fn check_memory() {
        let mut vector: Vec<u8> = Vec::new();

        for i in 0..10 {
            vector.push(i);
        }
        for _ in 0..10 {
            vector.pop();
        }

        use core::ptr;

        unsafe { 
            let ptr = vector.as_ptr().add(1);
            let data = ptr::read(ptr);
            assert_eq!(1, data);
        }    
    }
}

impl<T> FromIterator<T> for Vec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut temp_vec = Vec::new();
        for item in iter {
            temp_vec.push(item)
        }
        temp_vec
    }
}

impl<T> AsBytes for Vec<T> {}