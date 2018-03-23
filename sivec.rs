#![feature(alloc, core_intrinsics)]
extern crate alloc;
use alloc::raw_vec::RawVec;
use std::ops::Index;
use std::cell::RefCell;

enum Initializer<T> {
    None,
    Const(T),
    Closure(Box<Fn(usize) -> T>)
}

struct Value<T> {
    value: T,
    index: usize
}

// We are stuck with interior mutability by the definition
// of `Index::index`, which takes `self` as an immutable
// reference.

pub struct SIVec<T> {
    value_stack: RefCell<Vec<Value<T>>>,
    vec: RawVec<usize>,
    initializer: Initializer<T>
}
    
impl <T> SIVec<T> {
    pub fn new() -> SIVec<T> {
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: RawVec::new(),
            initializer: Initializer::None
        }
    }

    pub fn get_mut_ref<'a>(&'a self, index: usize, value: Option<T>)
                           -> &'a mut T {
        unimplemented!()
    }
}

impl <T: Clone> Index<usize> for SIVec<T> {
    type Output = T;

    fn index<'a>(&'a self, index: usize) -> &'a T {
        if index >= self.vec.cap() {
            panic!("SIVec: index bounds");
        }
        let store = self.vec.ptr();
        // XXX Need to do an unsafe read because
        // all we have is a raw pointer.
        let si = unsafe{*store.offset(index as isize)};
        let value_stack = self.value_stack.borrow();
        if si < value_stack.len() && value_stack[si].index == index {
            let result: *const T = &value_stack[si].value;
            // XXX The value is guaranteed to live as long
            // as the borrow of self, by construction of
            // this datatype.
            return unsafe{result.as_ref::<'a>()}.unwrap()
        }
        unimplemented!()
    }
}
