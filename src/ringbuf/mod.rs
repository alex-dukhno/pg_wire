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

//! Lock-free single-producer single-consumer (SPSC) FIFO ring buffer with direct access to inner data.
//!
//! # Overview
//!
//! `RingBuffer` is the initial structure representing ring buffer itself.
//! Ring buffer can be split into pair of `Producer` and `Consumer`.
//!
//! `Producer` and `Consumer` are used to append/remove elements to/from the ring buffer accordingly. They can be safely transferred between threads.
//! Operations with `Producer` and `Consumer` are lock-free - they're succeeded or failed immediately without blocking or waiting.
//!
//! Elements can be effectively appended/removed one by one or many at once.
//! Also data could be loaded/stored directly into/from [`Read`]/[`Write`] instances.
//! And finally, there are `unsafe` methods allowing thread-safe direct access in place to the inner memory being appended/removed.
//!
//! [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
//! [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
//!
//! When building with nightly toolchain it is possible to run benchmarks via `cargo bench --features benchmark`.
//!

#![rustfmt::skip]
#[cfg(test)]
mod tests;

mod consumer;
mod producer;
mod ring_buffer;

pub use consumer::*;
pub use producer::*;
pub use ring_buffer::*;
