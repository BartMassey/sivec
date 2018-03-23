#![feature(alloc, core_intrinsics)]
extern crate alloc;
use alloc::raw_vec::RawVec;
use std::ops::Index;
use std::cell::RefCell;

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
    vec: RawVec<usize>,
    initializer: Initializer<'a, T>
}
    
impl <'a, T: Clone> SIVec<'a, T> {
    pub fn new() -> SIVec<'a, T> {
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: RawVec::new(),
            initializer: Initializer::None
        }
    }

    pub fn new_with_default(default: T) -> SIVec<'a, T> {
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: RawVec::new(),
            initializer: Initializer::Const(default)
        }
    }

    pub fn new_with_constructor(constructor: &'a Fn(usize) -> T) -> SIVec<'a, T> {
        SIVec {
            value_stack: RefCell::new(Vec::new()),
            vec: RawVec::new(),
            initializer: Initializer::Closure(constructor)
        }
    }

    pub fn get_mut_ref(&'a self, index: usize, value: Option<T>)
                       -> &'a mut T {
        if index >= self.vec.cap() {
            panic!("SIVec: index bounds");
        }
        let store = self.vec.ptr();
        // XXX Need to do an unsafe read because
        // all we have is a raw pointer.
        let si = unsafe{*store.offset(index as isize)};
        let mut value_stack = self.value_stack.borrow_mut();
        let vsl = value_stack.len();
        if si < vsl && value_stack[si].index == index {
            let result: *mut T = &mut value_stack[si].value;
            if let Some(init) = value {
                // XXX Easier to just write through our pointer.
                unsafe{*result = init}
            }
            // XXX The value is guaranteed to live as long
            // as the borrow of self, by construction of
            // this datatype.
            return unsafe{result.as_mut::<'a>()}.unwrap()
        }
        let _init = match value {
            Some(v) => v,
            None => match self.initializer {
                Initializer::None => panic!("SIVec: unable to initialize"),
                Initializer::Const(ref v) => v.clone(),
                Initializer::Closure(ref f) => (*f)(index).clone()
            }
        };
        unimplemented!()
    }
}

impl <'a, T: Clone> Index<usize> for SIVec<'a, T> {
    type Output = T;

    fn index<'b>(&'b self, index: usize) -> &'b T {
        self.get_mut_ref(index, None)
    }
}
