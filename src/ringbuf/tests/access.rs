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

#[test]
fn push() {
    let cap = 2;
    let buf = RingBuffer::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vs_20 = (12, 34);
    let push_fn_20 = |left: &mut [u8], right: &mut [u8]| -> usize {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        left[0] = vs_20.0;
        left[1] = vs_20.1;
        2
    };

    assert_eq!(unsafe { prod.push_access(push_fn_20) }, 2);

    assert_eq!(cons.pop().unwrap(), vs_20.0);
    assert_eq!(cons.pop().unwrap(), vs_20.1);
    assert_eq!(cons.pop(), None);

    let vs_11 = (12, 34);
    let push_fn_11 = |left: &mut [u8], right: &mut [u8]| -> usize {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        left[0] = vs_11.0;
        right[0] = vs_11.1;
        2
    };

    assert_eq!(unsafe { prod.push_access(push_fn_11) }, 2);

    assert_eq!(cons.pop().unwrap(), vs_11.0);
    assert_eq!(cons.pop().unwrap(), vs_11.1);
    assert_eq!(cons.pop(), None);
}

#[test]
fn pop_full() {
    let cap = 2;
    let buf = RingBuffer::new(cap);
    let (_, mut cons) = buf.split();

    let dummy_fn = |_l: &mut [u8], _r: &mut [u8]| -> usize { 0 };
    assert_eq!(unsafe { cons.pop_access(dummy_fn) }, 0);
}

#[test]
fn pop_empty() {
    let cap = 2;
    let buf = RingBuffer::new(cap);
    let (_, mut cons) = buf.split();

    let dummy_fn = |_l: &mut [u8], _r: &mut [u8]| -> usize { 0 };
    assert_eq!(unsafe { cons.pop_access(dummy_fn) }, 0);
}

#[test]
fn pop() {
    let cap = 2;
    let buf = RingBuffer::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vs_20 = (12, 34);

    assert_eq!(prod.push(vs_20.0), Ok(()));
    assert_eq!(prod.push(vs_20.1), Ok(()));
    assert_eq!(prod.push(0), Err(0));

    let pop_fn_20 = |left: &mut [u8], right: &mut [u8]| -> usize {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            assert_eq!(left[0], vs_20.0);
            assert_eq!(left[1], vs_20.1);
            2
    };

    assert_eq!(unsafe { cons.pop_access(pop_fn_20) }, 2);

    let vs_11 = (12, 34);

    assert_eq!(prod.push(vs_11.0), Ok(()));
    assert_eq!(prod.push(vs_11.1), Ok(()));
    assert_eq!(prod.push(0), Err(0));

    let pop_fn_11 = |left: &mut [u8], right: &mut [u8]| -> usize {
            assert_eq!(left.len(), 1);
            assert_eq!(right.len(), 1);
            assert_eq!(left[0], vs_11.0);
            assert_eq!(right[0], vs_11.1);
            2
    };

    assert_eq!(unsafe { cons.pop_access(pop_fn_11) }, 2);
}

#[test]
fn push_return() {
    let cap = 2;
    let buf = RingBuffer::new(cap);
    let (mut prod, mut cons) = buf.split();

    let push_fn_0 = |left: &mut [u8], right: &mut [u8]| -> usize {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        0
    };

    assert_eq!(unsafe { prod.push_access(push_fn_0) }, 0);

    let push_fn_1 = |left: &mut [u8], right: &mut [u8]| -> usize {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        left[0] = 12;
        1
    };

    assert_eq!(unsafe { prod.push_access(push_fn_1) }, 1);

    let push_fn_2 = |left: &mut [u8], right: &mut [u8]| -> usize {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 0);
        left[0] = 34;
        1
    };

    assert_eq!(unsafe { prod.push_access(push_fn_2) }, 1);

    assert_eq!(cons.pop().unwrap(), 12);
    assert_eq!(cons.pop().unwrap(), 34);
    assert_eq!(cons.pop(), None);
}

#[test]
fn pop_return() {
    let cap = 2;
    let buf = RingBuffer::new(cap);
    let (mut prod, mut cons) = buf.split();

    assert_eq!(prod.push(12), Ok(()));
    assert_eq!(prod.push(34), Ok(()));
    assert_eq!(prod.push(0), Err(0));

    let pop_fn_0 = |left: &mut [u8], right: &mut [u8]| -> usize {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        0
    };

    assert_eq!(unsafe { cons.pop_access(pop_fn_0) }, 0);

    let pop_fn_1 = |left: &mut [u8], right: &mut [u8]| -> usize {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            assert_eq!(left[0], 12);
            1
    };

    assert_eq!(unsafe { cons.pop_access(pop_fn_1) }, 1);

    let pop_fn_2 = |left: &mut [u8], right: &mut [u8]| -> usize {
            assert_eq!(left.len(), 1);
            assert_eq!(right.len(), 0);
            assert_eq!(left[0], 34);
            1
    };

    assert_eq!(unsafe { cons.pop_access(pop_fn_2) }, 1);
}

#[test]
fn push_pop() {
    let cap = 2;
    let buf = RingBuffer::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vs_20 = (12, 34);
    let push_fn_20 = |left: &mut [u8], right: &mut [u8]| -> usize {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        left[0] = vs_20.0;
        left[1] = vs_20.1;
        2
    };

    assert_eq!(unsafe { prod.push_access(push_fn_20) }, 2);

    let pop_fn_20 = |left: &mut [u8], right: &mut [u8]| -> usize {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            assert_eq!(left[0], vs_20.0);
            assert_eq!(left[1], vs_20.1);
            2
    };

    assert_eq!(unsafe { cons.pop_access(pop_fn_20) }, 2);

    let vs_11 = (12, 34);
    let push_fn_11 = |left: &mut [u8], right: &mut [u8]| -> usize {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        left[0] = vs_11.0;
        right[0] = vs_11.1;
        2
    };

    assert_eq!(unsafe { prod.push_access(push_fn_11) }, 2);

    let pop_fn_11 = |left: &mut [u8], right: &mut [u8]| -> usize {
            assert_eq!(left.len(), 1);
            assert_eq!(right.len(), 1);
            assert_eq!(left[0], vs_11.0);
            assert_eq!(right[0], vs_11.1);
            2
    };

    assert_eq!(unsafe { cons.pop_access(pop_fn_11) }, 2);
}

#[test]
fn discard() {
    // Initialize ringbuffer, prod and cons
    let rb = RingBuffer::new(10);
    let (mut prod, mut cons) = rb.split();
    let mut i = 0;

    // Fill the buffer
    for _ in 0..10 {
        prod.push(i).unwrap();
        i += 1;
    }

    // Pop in the middle of the buffer
    assert_eq!(cons.discard(5), 5);

    // Make sure changes are taken into account
    assert_eq!(cons.pop().unwrap(), 5);

    // Fill the buffer again
    for _ in 0..5 {
        prod.push(i).unwrap();
        i += 1;
    }

    assert_eq!(cons.discard(6), 6);
    assert_eq!(cons.pop().unwrap(), 12);

    // Fill the buffer again
    for _ in 0..7 {
        prod.push(i).unwrap();
        i += 1;
    }

    // Ask too much, delete the max number of elements
    assert_eq!(cons.discard(10), 9);

    // Try to remove more than possible
    assert_eq!(cons.discard(1), 0);

    // Make sure it is still usable
    assert_eq!(cons.pop(), None);
    assert_eq!(prod.push(0), Ok(()));
    assert_eq!(cons.pop(), Some(0));
}
