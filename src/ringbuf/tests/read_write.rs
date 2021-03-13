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
use std::io;

#[test]
fn from() {
    let buf0 = RingBuffer::new(4);
    let buf1 = RingBuffer::new(4);
    let (mut prod0, mut cons0) = buf0.split();
    let (mut prod1, mut cons1) = buf1.split();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2]), 3);

    match prod1.read_from(&mut cons0, None) {
        Ok(n) => assert_eq!(n, 3),
        other => panic!("{:?}", other),
    }
    match prod1.read_from(&mut cons0, None) {
        Err(e) => {
            assert_eq!(e.kind(), io::ErrorKind::WouldBlock);
        }
        other => panic!("{:?}", other),
    }

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [0, 1, 2]);

    assert_eq!(prod0.push_slice(&[3, 4, 5]), 3);

    match prod1.read_from(&mut cons0, None) {
        Ok(n) => assert_eq!(n, 2),
        other => panic!("{:?}", other),
    }
    assert_eq!(cons1.pop_slice(&mut tmp), 2);
    assert_eq!(tmp[0..2], [3, 4]);

    match prod1.read_from(&mut cons0, None) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }
    assert_eq!(cons1.pop_slice(&mut tmp), 1);
    assert_eq!(tmp[0..1], [5]);

    assert_eq!(prod1.push_slice(&[6, 7, 8]), 3);
    assert_eq!(prod0.push_slice(&[9, 10]), 2);

    match prod1.read_from(&mut cons0, None) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }
    match prod1.read_from(&mut cons0, None) {
        Ok(n) => assert_eq!(n, 0),
        other => panic!("{:?}", other),
    }

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [6, 7, 8, 9]);
}

#[test]
fn into() {
    let buf0 = RingBuffer::new(4);
    let buf1 = RingBuffer::new(4);
    let (mut prod0, mut cons0) = buf0.split();
    let (mut prod1, mut cons1) = buf1.split();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2]), 3);

    match cons0.write_into(&mut prod1, None) {
        Ok(n) => assert_eq!(n, 3),
        other => panic!("{:?}", other),
    }
    match cons0.write_into(&mut prod1, None) {
        Ok(n) => assert_eq!(n, 0),
        other => panic!("{:?}", other),
    }

    assert_eq!(cons1.pop_slice(&mut tmp), 3);
    assert_eq!(tmp[0..3], [0, 1, 2]);

    assert_eq!(prod0.push_slice(&[3, 4, 5]), 3);

    match cons0.write_into(&mut prod1, None) {
        Ok(n) => assert_eq!(n, 2),
        other => panic!("{:?}", other),
    }
    assert_eq!(cons1.pop_slice(&mut tmp), 2);
    assert_eq!(tmp[0..2], [3, 4]);

    match cons0.write_into(&mut prod1, None) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }
    assert_eq!(cons1.pop_slice(&mut tmp), 1);
    assert_eq!(tmp[0..1], [5]);

    assert_eq!(prod1.push_slice(&[6, 7, 8]), 3);
    assert_eq!(prod0.push_slice(&[9, 10]), 2);

    match cons0.write_into(&mut prod1, None) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }
    match cons0.write_into(&mut prod1, None) {
        Err(e) => {
            assert_eq!(e.kind(), io::ErrorKind::WouldBlock);
        }
        other => panic!("{:?}", other),
    }

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [6, 7, 8, 9]);
}

#[test]
fn count() {
    let buf0 = RingBuffer::new(4);
    let buf1 = RingBuffer::new(4);
    let (mut prod0, mut cons0) = buf0.split();
    let (mut prod1, mut cons1) = buf1.split();

    let mut tmp = [0; 5];

    assert_eq!(prod0.push_slice(&[0, 1, 2, 3]), 4);

    match prod1.read_from(&mut cons0, Some(3)) {
        Ok(n) => assert_eq!(n, 3),
        other => panic!("{:?}", other),
    }
    match prod1.read_from(&mut cons0, Some(2)) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [0, 1, 2, 3]);

    assert_eq!(prod0.push_slice(&[4, 5, 6, 7]), 4);

    match cons0.write_into(&mut prod1, Some(3)) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }
    match cons0.write_into(&mut prod1, Some(2)) {
        Ok(n) => assert_eq!(n, 2),
        other => panic!("{:?}", other),
    }
    match cons0.write_into(&mut prod1, Some(2)) {
        Ok(n) => assert_eq!(n, 1),
        other => panic!("{:?}", other),
    }

    assert_eq!(cons1.pop_slice(&mut tmp), 4);
    assert_eq!(tmp[0..4], [4, 5, 6, 7]);
}
