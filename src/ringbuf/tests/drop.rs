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

use super::*;

/// Test the `pop_each` with internal function that returns false
#[test]
fn pop_each_test1() {
    let cap = 10usize;
    let (mut producer, mut consumer) = RingBuffer::new(cap).split();

    for i in 0..cap {
        producer.push(i as u8).unwrap();
    }

    for _ in 0..cap {
        let removed = consumer.pop_each(|_val| -> bool { false }, None);
        assert_eq!(removed, 1);
    }

    assert_eq!(consumer.len(), 0);
}

/// Test the `pop_each` with capped pop
#[test]
fn pop_each_test2() {
    let cap = 10usize;
    let (mut producer, mut consumer) = RingBuffer::new(cap).split();

    for i in 0..cap {
        producer.push(i as u8).unwrap();
    }

    for _ in 0..cap {
        let removed = consumer.pop_each(|_val| -> bool { true }, Some(1));
        assert_eq!(removed, 1);
    }

    assert_eq!(consumer.len(), 0);
}

/// Test the `push_each` with internal function that adds only 1 element.
#[test]
fn push_each_test1() {
    let cap = 10usize;
    let (mut producer, mut consumer) = RingBuffer::new(cap).split();

    for i in 0..cap {
        let mut count = 0;
        // Add 1 element at a time
        let added = producer.push_each(|| -> Option<u8> {
            if count == 0 {
                count += 1;
                Some(i as u8)
            } else {
                None
            }
        });
        assert_eq!(added, 1);
    }

    for _ in 0..cap {
        consumer.pop().unwrap();
    }

    assert_eq!(consumer.len(), 0);
}

/// Test the `push_each` with split internal buffer
#[test]
fn push_each_test2() {
    let cap = 10usize;
    let cap_half = 5usize;
    let (mut producer, mut consumer) = RingBuffer::new(cap).split();

    // Fill the entire buffer
    for i in 0..cap {
        producer.push(i as u8).unwrap();
    }

    // Remove half elements
    for _ in 0..cap_half {
        consumer.pop().unwrap();
    }

    // Re add half elements one by one and check the return count.
    for i in 0..cap_half {
        let mut count = 0;
        // Add 1 element at a time
        let added = producer.push_each(|| -> Option<u8> {
            if count == 0 {
                count += 1;
                Some(i as u8)
            } else {
                None
            }
        });
        assert_eq!(added, 1);
    }

    for _ in 0..cap {
        consumer.pop().unwrap();
    }

    assert_eq!(consumer.len(), 0);
}
