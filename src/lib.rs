// Copyright © 2018 Bart Massey
// [This program is licensed under the "MIT License"]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

//! A "self-initializing" vector. This data structure offers
//! O(1) indexing and O(1) initialization: an element is
//! lazily initialized upon first reference as needed.
//!
//! # Theory of Operation
//!
//! The implementation uses a large index vector of
//! initially uninitialized memory, together with a stack of
//! stored values. As such, it will occupy space
//! proportional to its capacity, and additional space
//! proportional to the number of stored elements.
//!
//! The basic strategy of this data structure is to keep an
//! initially-uninitialized index vector and a stack of
//! allocated values.  A given index has a valid value if
//! its index vector points into the stack and the stack
//! element it points to shows the same index. Otherwise,
//! the data structure can be adjusted to make this true,
//! creating a default value as needed.
//!
//! ## Data Structure
//! 
//! * We have a stack, which is initially empty.
//! 
//! * Every entry on the stack points to a specific cell in
//!   the array.
//! 
//! * Every initialized entry in the array points back to
//! its corresponding entry on the stack. (The uninitialized
//! entries, obviously, could point anywhere, including onto
//! the stack.)
//! 
//! ## To read from the array:
//! 
//! * Look up the array entry. See if it points back into
//! the stack. If so, *check that the stack entry points to
//! that array entry.*
//! 
//!    * If both pointers are valid and match, the array
//!    element has been previously initialized, so its
//!    contents are valid. In this case, finish the
//!    read. (The value can be stored either on the stack
//!    or in the array: the stack is a better idea, since
//!    the array uses vast amounts of VM and the stack will
//!    be limited-size.)
//!  
//!    * If the array pointer is invalid or the stack
//!    pointer doesn't match it, the array element is
//!    uninitialized. Throw an error. (Alternatively,
//!    initialize as below with some default value and
//!    return that.)
//! 
//! ## To write to the array:
//! 
//! * Do the same check as the read. If the check passes, overwrite the value.
//! 
//! * If the check fails, push a new entry on the stack,
//! adjust the stack entry and the array entry to point to
//! each other, and write the value.
//! 
//! ## Efficiency
//! 
//! Note that every operation is constant-time and consumes
//! constant space. (We will agree to ignore the giant pile
//! of uninitialized virtual memory lying in the corner.)
//! Thus, our efficiency is as good (in some sense) as a
//! normal array write. But *we don't have to initialize the
//! giant array first,* which is great if the array is going
//! to be really sparsely filled.

use std::cell::RefCell;
use std::isize;
use std::ops::{Index, IndexMut};

struct Value<T> {
    value: T,
    index: usize,
}


/// A "self-initializing" vector.
pub struct SIVec<T> {
    // We are stuck with interior mutability by the definition
    // of `Index::index`, which takes `self` as an immutable
    // reference.
    value_stack: RefCell<Vec<Value<T>>>,
    vec: Vec<usize>,
    initializer: Box<dyn Fn(usize) -> T + 'static>,
}

impl<T> SIVec<T> {
    /// Create a new `SIVec` with the given (fixed)
    /// capacity.  Since no initialization is provided, if a
    /// given index is read before first write the access
    /// will panic. The maximum allowed capacity is `std::isize::MAX`.
    ///
    /// # Panics
    ///
    /// Will panic with a failed assertion if called with a
    /// capacity exceeding the allowed bound.
    pub fn new(cap: usize) -> SIVec<T> {
        assert!(cap <= isize::MAX as usize);
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: Vec::with_capacity(cap),
            initializer: Box::new(|_| {
                panic!("no initializer for SIVec")
            }),
        }
    }

    /// Create a new `SIVec` with the given (fixed)
    /// capacity. If a given index is read before first
    /// write, a clone of the given default value will be
    /// supplied.  The maximum allowed capacity is
    /// `std::isize::MAX`.
    ///
    /// # Panics
    ///
    /// Will panic with a failed assertion if called with a
    /// capacity exceeding the allowed bound.
    pub fn with_init(cap: usize, value: T) -> SIVec<T>
    where
        T: Clone + 'static,
    {
        assert!(cap <= isize::MAX as usize);
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: Vec::with_capacity(cap),
            initializer: Box::new(move |_| value.clone()),
        }
    }

    /// Create a new `SIVec` with the given (fixed)
    /// capacity. If a given index `i` is read before first
    /// write, the `init_fn` will be called with `i` to get
    /// a default value.  The maximum allowed capacity is
    /// `std::isize::MAX`.
    ///
    /// # Panics
    ///
    /// Will panic with a failed assertion if called with a
    /// capacity exceeding the allowed bound.
    pub fn with_init_fn<F>(cap: usize, init_fn: F) -> SIVec<T>
    where
        F: Fn(usize) -> T + 'static,
    {
        assert!(cap <= isize::MAX as usize);
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: Vec::with_capacity(cap),
            initializer: Box::new(init_fn),
        }
    }

    // The heart of all this mess. This function will return
    // a mutable reference to storage for a value stored
    // notionally at the given `index`.
    //
    // In the case that this is a reference to a
    // previously-uninitialized location, the behavior of
    // this function will depend on the `value` argument. If
    // value is `None`, the storage will be initialized with
    // a value obtained from the `self` initializer,
    // panicking if no initializer was provided. Otherwise,
    // the storage will be initialized with the given value.
    fn get_mut_ref(&self, index: usize, value: Option<T>) -> *mut T {
        if index >= self.vec.capacity() {
            panic!("SIVec: index bounds");
        }
        let store = self.vec.as_ptr() as *mut usize;
        // This offset will not overflow. The capacity is
        // guaranteed to be less than `isize::MAX` by the
        // constructors, and we have checked the bound
        // above.
        let ip = unsafe { store.add(index) };
        // XXX Need to do an unsafe read because
        // all we have is a raw pointer.
        // XXX Miri is not happy with this read, since
        // the memory is known-undefined. I don't think
        // there's much to be done about this given the
        // current Rust UB rules.
        let si = unsafe { *ip };
        let mut value_stack = self.value_stack.borrow_mut();
        let vsl = value_stack.len();
        if si < vsl && value_stack[si].index == index {
            let vp = &mut value_stack[si].value;
            if let Some(value) = value {
                *vp = value;
            }
            // XXX The value is guaranteed to live as long
            // as the borrow of self, by construction of
            // this datatype.
            return vp;
        }
        let value = match value {
            None => (*self.initializer)(index),
            Some(value) => value,
        };
        let value = Value { value, index };
        value_stack.push(value);
        // XXX Initialize the index.
        unsafe { *ip = vsl };
        &mut value_stack[vsl].value
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
    /// let mut v = sivec::SIVec::new(12);
    /// v.set(3, 'a');
    /// assert_eq!(v[3], 'a');
    /// ```
    pub fn set(&mut self, index: usize, value: T) {
        let _ = self.get_mut_ref(index, Some(value));
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
        let ptr = self.get_mut_ref(index, None);
        unsafe { ptr.as_ref() }.unwrap()
    }

    /// Report the capacity of this structure.
    pub fn cap(&self) -> usize {
        self.vec.capacity()
    }
}

impl<T> Index<usize> for SIVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        let ptr = self.get_mut_ref(index, None);
        unsafe { ptr.as_ref() }.unwrap()
    }
}

impl<T> IndexMut<usize> for SIVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        // XXX Since we can't know whether the caller
        // will initialize the value, we need to
        // provide a default value before returning.
        let ptr = self.get_mut_ref(index, None);
        unsafe { ptr.as_mut() }.unwrap()
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
