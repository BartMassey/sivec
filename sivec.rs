#![feature(alloc)]
// Copyright © 2018 Bart Massey
// [This program is licensed under the "MIT License"]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

//! A "self-initializing" vector. This data structure offers
//! O(1) indexing and O(1) initialization: an element is
//! lazily initialized upon first reference as needed.
//!
//! The implementation uses a large index vector of
//! initially uninitialized memory, together with a stack of
//! stored values. As such, it will occupy space proportional
//! to its capacity, and additional space proportional to the
//! number of stored elements.

extern crate alloc;
use alloc::raw_vec::RawVec;
use std::ops::{Index, IndexMut};
use std::cell::RefCell;
use std::mem;

struct Value<T> {
    value: T,
    index: usize
}

// The basic strategy of this data structure is to keep a
// `RawVec` index vector and a stack of allocated values.  A
// given index has a valid value if its index vector points
// into the stack and the stack element it points to shows
// the same index. Otherwise, the data structure can be
// adjusted to make this true, creating a default value as
// needed.

/// A "self-initializing" vector.
pub struct SIVec<T> {
    // We are stuck with interior mutability by the definition
    // of `Index::index`, which takes `self` as an immutable
    // reference.
    value_stack: RefCell<Vec<Value<T>>>,
    vec: RawVec<usize>,
    initializer: Box<Fn(usize)->T + 'static>,
}
    
impl <T> SIVec<T> {

    /// Create a new `SIVec` with the given (fixed)
    /// capacity.  Since no initialization is provided, if a
    /// given index is read before first write the access
    /// will panic.
    pub fn new(cap: usize) -> SIVec<T> {
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: RawVec::with_capacity(cap),
            initializer: Box::new(|_| panic!("no initializer for SIVec")),
        }
    }

    /// Create a new `SIVec` with the given (fixed)
    /// capacity. If a given index is read before first write,
    /// a clone of the given default value will be supplied.
    pub fn with_init(cap: usize, value: T) -> SIVec<T>
     where T: Clone, T: 'static {
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: RawVec::with_capacity(cap),
            initializer: Box::new(move |_| value.clone()),
        }
    }

    /// Create a new `SIVec` with the given (fixed)
    /// capacity. If a given index `i` is read before first
    /// write, the `init_fn` will be called with `i` to get
    /// a default value.
    pub fn with_init_fn<F>(cap: usize, init_fn: F) -> SIVec<T>
     where F: Fn(usize)->T + 'static {
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: RawVec::with_capacity(cap),
            initializer: Box::new(init_fn),
        }
    }

    // The heart of all this mess. This function will return
    // either a mutable reference to an existing value stored
    // notionally at the given `index`, or will allocate
    // a new value and return a mutable reference to that.
    // In the second case, if `need_default` is true, this
    // function will instead panic.
    fn get_mut_ref(&self, index: usize, need_default: bool)
                   -> &mut T {
        if index >= self.vec.cap() {
            panic!("SIVec: index bounds");
        }
        let store = self.vec.ptr();
        let ip = unsafe{store.offset(index as isize)};
        // XXX Need to do an unsafe read because
        // all we have is a raw pointer.
        let si = unsafe{*ip};
        let mut value_stack = self.value_stack.borrow_mut();
        let vsl = value_stack.len();
        if si < vsl && value_stack[si].index == index {
            let result: *mut T = &mut value_stack[si].value;
            // XXX The value is guaranteed to live as long
            // as the borrow of self, by construction of
            // this datatype.
            return unsafe{result.as_mut()}.unwrap()
        }
        let init =
            if need_default {
                (*self.initializer)(index)
            } else {
                // XXX The caller is committed to immediately
                // initializing this cell.
                unsafe{mem::uninitialized()}
            };
        let new_value = Value {
            value: init,
            index: index
        };
        value_stack.push(new_value);
        // XXX Initialize the index.
        unsafe{*ip = vsl};
        let result: *mut T = &mut value_stack[vsl].value;
        // XXX See existing case above.
        return unsafe{result.as_mut()}.unwrap()
    }

    /// Set the given location to have the given value.
    /// This is potentially more efficient than storing
    /// through an index: in the index case, a default value
    /// will be created and then immediately replaced.
    /// For the same reason, this is the only way to
    /// initially set a value at a given index when no
    /// default has been supplied.
    ///
    /// # Examples
    /// 
    /// ```
    /// let v = sivec::SIVec::new(12);
    /// v.set(3, 'a');
    /// assert_eq!(v[3], 'a');
    /// ```
    pub fn set(&self, index: usize, value: T) {
        let v = self.get_mut_ref(index, false);
        *v = value;
    }

    /// Get an immutable reference to the location holding
    /// the given value. When applied to an uninitialized
    /// index, this function will store a default value
    /// there, or panic if this is not possible.
    ///
    /// It is usually more convenient to use indexing
    /// than this function.
    /// 
    /// # Examples
    /// 
    /// ```
    /// let v = sivec::SIVec::with_init(12, 'a');
    /// assert_eq!(*v.get(3), 'a');
    /// ```
    pub fn get(&self, index: usize) -> &T {
        self.get_mut_ref(index, true)
    }

    /// Report the capacity of this structure.
    pub fn cap(&self) -> usize {
        self.vec.cap()
    }
}

impl <T> Index<usize> for SIVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        self.get_mut_ref(index, true)
    }
}

impl <T> IndexMut<usize> for SIVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        // XXX Since we can't know whether the caller
        // will initialize the value, we need to
        // provide a default value before returning.
        self.get_mut_ref(index, true)
    }
}

#[test]
fn basic_test() {
    let mut v = SIVec::new(10);
    v.set(3, 'a');
    assert_eq!(v[3], 'a');
    v[3] = 'b';
    assert_eq!(v[3], 'b');

    let mut v = SIVec::with_init(10, 'b');
    v[3] = 'a';
    assert_eq!(v[3], 'a');
    assert_eq!(v[4], 'b');
    v[4] = 'c';
    assert_eq!(v[3], 'a');
    assert_eq!(v[4], 'c');
    assert_eq!(v[5], 'b');
    v[3] = 'b';
    assert_eq!(v[3], 'b');
    assert_eq!(v[4], 'c');
    assert_eq!(v[5], 'b');
    assert_eq!(v[6], 'b');

    let init = |i| std::char::from_u32('a' as u32 + i as u32).unwrap();
    let v = SIVec::with_init_fn(10, init);
    assert_eq!(v[0], 'a');
    assert_eq!(v[2], 'c');
}
