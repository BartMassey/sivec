# SIVec: Rust "self-initializing" vector
Copyright (c) 2018 Bart Massey

This crate implements a "self-initializing" vector.  A
self-initializing vector stores values at a sparse
collection of indices, using storage linear in the number of
stored values. Values are created and initialized on first
reference.

The basic idea is to use an uninitialized array of indices
into a stack of index-value pairs. When the array of indices
is referenced, a check is done to see if the stack index is
valid. If so, the value on the stack is used; otherwise a
new pair is pushed onto the stack and the indices array is
set to point at it.

I was told about this data structure in grad school at some
point, but don't have a reference handy. If someone else
does it would be appreciated.

This code was written as much as an exercise in interior
mutability as because it would be useful for
anything. It has not been extensively tested, and not
benchmarked at all.

This program is licensed under the "MIT License".  Please
see the file LICENSE in the source distribution of this
software for license terms.
