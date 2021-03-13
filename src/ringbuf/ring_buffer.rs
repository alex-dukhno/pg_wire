// Copyright (c) 2019 Alexey Gerasev
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::{sync::Arc, vec::Vec};
use core::{
    cell::UnsafeCell,
    cmp::min,
    ptr::{self, copy},
    sync::atomic::{AtomicUsize, Ordering},
};
use super::{consumer::Consumer, producer::Producer};

pub(crate) struct SharedVec {
    cell: UnsafeCell<Vec<u8>>,
}

unsafe impl Sync for SharedVec {}

impl SharedVec {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            cell: UnsafeCell::new(data),
        }
    }

    pub unsafe fn get_ref(&self) -> &Vec<u8> {
        &*self.cell.get()
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut(&self) -> &mut Vec<u8> {
        &mut *self.cell.get()
    }
}

/// Ring buffer itself.
pub struct RingBuffer {
    pub(crate) data: SharedVec,
    pub(crate) head: AtomicUsize,
    pub(crate) tail: AtomicUsize,
}

impl RingBuffer {
    /// Creates a new instance of a ring buffer.
    pub fn new(capacity: usize) -> RingBuffer {
        let mut data = Vec::new();
        data.resize_with(capacity + 1, || 0);
        RingBuffer {
            data: SharedVec::new(data),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Splits ring buffer into producer and consumer.
    pub fn split(self) -> (Producer, Consumer) {
        let arc = Arc::new(self);
        (Producer { rb: arc.clone() }, Consumer { rb: arc })
    }

    /// Returns capacity of the ring buffer.
    pub fn capacity(&self) -> usize {
        unsafe { self.data.get_ref() }.len() - 1
    }

    /// Checks if the ring buffer is empty.
    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        head == tail
    }

    /// Checks if the ring buffer is full.
    pub fn is_full(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        (tail + 1) % (self.capacity() + 1) == head
    }

    /// The length of the data in the buffer.
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        (tail + self.capacity() + 1 - head) % (self.capacity() + 1)
    }

    /// The remaining space in the buffer.
    pub fn remaining(&self) -> usize {
        self.capacity() - self.len()
    }
}

struct SlicePtr {
    pub ptr: *mut u8,
    pub len: usize,
}

impl SlicePtr {
    fn null() -> SlicePtr {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
        }
    }

    fn new(slice: &mut [u8]) -> SlicePtr {
        Self {
            ptr: slice.as_mut_ptr(),
            len: slice.len(),
        }
    }

    unsafe fn shift(&mut self, count: usize) {
        self.ptr = self.ptr.add(count);
        self.len -= count;
    }
}

/// Moves at most `count` items from the `src` consumer to the `dst` producer.
/// Consumer and producer may be of different buffers as well as of the same one.
///
/// `count` is the number of items being moved, if `None` - as much as possible items will be moved.
///
/// Returns number of items been moved.
pub fn move_items(src: &mut Consumer, dst: &mut Producer, count: Option<usize>) -> usize {
    unsafe {
        src.pop_access(|src_left, src_right| -> usize {
            dst.push_access(|dst_left, dst_right| -> usize {
                let n = count.unwrap_or_else(|| {
                    min(
                        src_left.len() + src_right.len(),
                        dst_left.len() + dst_right.len(),
                    )
                });
                let mut m = 0;
                let mut src = (SlicePtr::new(src_left), SlicePtr::new(src_right));
                let mut dst = (SlicePtr::new(dst_left), SlicePtr::new(dst_right));

                loop {
                    let k = min(n - m, min(src.0.len, dst.0.len));
                    if k == 0 {
                        break;
                    }
                    copy(src.0.ptr, dst.0.ptr, k);
                    if src.0.len == k {
                        src.0 = src.1;
                        src.1 = SlicePtr::null();
                    } else {
                        src.0.shift(k);
                    }
                    if dst.0.len == k {
                        dst.0 = dst.1;
                        dst.1 = SlicePtr::null();
                    } else {
                        dst.0.shift(k);
                    }
                    m += k
                }

                m
            })
        })
    }
}
