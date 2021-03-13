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
fn for_each() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    prod.push(10).unwrap();
    prod.push(20).unwrap();

    let mut sum_1 = 0;
    cons.for_each(|v| {
        sum_1 += *v;
    });

    let first = cons.pop().expect("First element not available");
    let second = cons.pop().expect("Second element not available");

    assert_eq!(sum_1, first + second);
}

#[test]
fn for_each_mut() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    prod.push(10).unwrap();
    prod.push(20).unwrap();

    cons.for_each_mut(|v| {
        *v *= 2;
    });

    let mut sum_1 = 0;
    cons.for_each_mut(|v| {
        sum_1 += *v;
    });

    let first = cons.pop().expect("First element not available");
    let second = cons.pop().expect("Second element not available");

    assert_eq!(sum_1, first + second);
}

#[test]
fn push_pop_slice() {
    let buf = RingBuffer::<i32>::new(4);
    let (mut prod, mut cons) = buf.split();

    let mut tmp = [0; 5];

    assert_eq!(prod.push_slice(&[]), 0);
    assert_eq!(prod.push_slice(&[0, 1, 2]), 3);

    assert_eq!(cons.pop_slice(&mut tmp[0..2]), 2);
    assert_eq!(tmp[0..2], [0, 1]);

    assert_eq!(prod.push_slice(&[3, 4]), 2);
    assert_eq!(prod.push_slice(&[5, 6]), 1);

    assert_eq!(cons.pop_slice(&mut tmp[0..3]), 3);
    assert_eq!(tmp[0..3], [2, 3, 4]);

    assert_eq!(prod.push_slice(&[6, 7, 8, 9]), 3);

    assert_eq!(cons.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [5, 6, 7, 8]);
}

#[test]
fn move_slice() {
    let buf0 = RingBuffer::<i32>::new(4);
    let buf1 = RingBuffer::<i32>::new(4);
    let (mut prod0, mut cons0) = buf0.split();
    let (mut prod1, mut cons1) = buf1.split();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2]), 3);

    assert_eq!(prod1.move_from(&mut cons0, None), 3);
    assert_eq!(prod1.move_from(&mut cons0, None), 0);

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [0, 1, 2]);

    assert_eq!(prod0.push_slice(&[3, 4, 5]), 3);

    assert_eq!(prod1.move_from(&mut cons0, None), 3);

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [3, 4, 5]);

    assert_eq!(prod1.push_slice(&[6, 7, 8]), 3);
    assert_eq!(prod0.push_slice(&[9, 10]), 2);

    assert_eq!(prod1.move_from(&mut cons0, None), 1);
    assert_eq!(prod1.move_from(&mut cons0, None), 0);

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [6, 7, 8, 9]);
}

#[test]
fn move_slice_count() {
    let buf0 = RingBuffer::<i32>::new(4);
    let buf1 = RingBuffer::<i32>::new(4);
    let (mut prod0, mut cons0) = buf0.split();
    let (mut prod1, mut cons1) = buf1.split();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2]), 3);

    assert_eq!(prod1.move_from(&mut cons0, Some(2)), 2);

    assert_eq!(cons1.pop_slice(&mut tmp), 2);
    assert_eq!(tmp[0..2], [0, 1]);

    assert_eq!(prod1.move_from(&mut cons0, Some(2)), 1);

    assert_eq!(cons1.pop_slice(&mut tmp), 1);
    assert_eq!(tmp[0..1], [2]);

    assert_eq!(prod0.push_slice(&[3, 4, 5, 6]), 4);

    assert_eq!(prod1.move_from(&mut cons0, Some(3)), 3);

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [3, 4, 5]);

    assert_eq!(prod0.push_slice(&[7, 8, 9]), 3);

    assert_eq!(prod1.move_from(&mut cons0, Some(5)), 4);

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [6, 7, 8, 9]);
}
