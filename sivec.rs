#![feature(alloc, allocator_api, core_intrinsics)]
extern crate alloc;
use alloc::raw_vec::RawVec;
use alloc::heap::Heap;
use std::ops::{Index, IndexMut};
use std::cell::RefCell;
use std::mem;

enum Initializer<'a, T: 'a + Clone> {
    None,
    Const(T),
    Closure(&'a Fn(usize) -> T)
}

struct Value<T> {
    value: T,
    index: usize
}

// We are stuck with interior mutability by the definition
// of `Index::index`, which takes `self` as an immutable
// reference.

pub struct SIVec<'a, T: 'a + Clone> {
    value_stack: RefCell<Vec<Value<T>>>,
    vec: RawVec<usize, Heap>,
    initializer: Initializer<'a, T>
}
    
impl <'a, T: Clone> SIVec<'a, T> {
    pub fn new(capacity: usize) -> SIVec<'a, T> {
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: RawVec::with_capacity(capacity),
            initializer: Initializer::None
        }
    }

    pub fn with_default(capacity: usize, default: T)
                        -> SIVec<'a, T> {
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: RawVec::with_capacity(capacity),
            initializer: Initializer::Const(default)
        }
    }

    pub fn with_constructor(capacity: usize,
                            constructor: &'a Fn(usize) -> T)
                            -> SIVec<'a, T> {
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: RawVec::with_capacity(capacity),
            initializer: Initializer::Closure(constructor)
        }
    }

    fn get_mut_ref(&'a self, index: usize, need_default: bool)
                   -> &'a mut T {
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
            return unsafe{result.as_mut::<'a>()}.unwrap()
        }
        let init =
            if need_default {
                match self.initializer {
                    Initializer::None =>
                        panic!("SIVec: unable to initialize"),
                    Initializer::Const(ref v) =>
                        v.clone(),
                    Initializer::Closure(ref f) =>
                        (*f)(index).clone()
                }
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
        return unsafe{result.as_mut::<'a>()}.unwrap()
    }

    pub fn set(&self, index: usize, value: T) {
        let v = self.get_mut_ref(index, false);
        *v = value;
    }

    pub fn get(&self, index: usize) -> &T {
        self.get_mut_ref(index, true)
    }
}

impl <'a, T: Clone> Index<usize> for SIVec<'a, T> {
    type Output = T;

    fn index<'b>(&'b self, index: usize) -> &'b T {
        self.get_mut_ref(index, true)
    }
}

impl <'a, T: Clone> IndexMut<usize> for SIVec<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        // XXX Since we can't know whether the caller
        // will initialize the value, we need to
        // provide a default value before returning.
        self.get_mut_ref(index, true)
    }
}

#[cfg(test)]
use std::char;

#[test]
fn basic_test() {
    let mut v = SIVec::new(10);
    v.set(3, 'a');
    assert_eq!(v[3], 'a');
    v[3] = 'b';
    assert_eq!(v[3], 'b');

    let mut v = SIVec::with_default(10, 'b');
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

    let init = |i| char::from_u32('a' as u32 + i as u32).unwrap();
    let v = SIVec::with_constructor(10, &init);
    assert_eq!(v[0], 'a');
    assert_eq!(v[2], 'c');
}
