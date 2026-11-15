// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Direct tests for the internal buffer/DNS safety layer.  This compiles the
// same source that libc includes, so the tests can run without the C ABI and
// are suitable for Miri.

#[path = "../libc/src/safety.rs"]
mod safety;

use safety::{dns_expand_name_at, dns_skip_name, InBuf, NameWriter, OutBuf};

fn view(bytes: &[u8]) -> InBuf {
    unsafe { InBuf::new(bytes.as_ptr(), bytes.len()).unwrap() }
}

#[test]
fn inbuf_reads_within_bounds() {
    let data = [10u8, 0x12, 0x34, 0xff];
    let buf = view(&data);
    assert_eq!(buf.len(), 4);
    assert_eq!(buf.peek_u8(0), Some(10));
    assert_eq!(buf.peek_u16_be(1), Some(0x1234));
    assert_eq!(buf.peek_u8(4), None);
    assert_eq!(buf.peek_u16_be(3), None);
    assert_eq!(buf.peek_u32_be(0), Some(0x0a1234ff));
    assert_eq!(buf.peek_u32_be(1), None);
    assert_eq!(buf.slice_at(1, 3), Some(&data[1..]));
    assert_eq!(buf.slice_at(1, 4), None);
    assert_eq!(buf.slice_at(4, 0), Some(&[][..]));

    let empty = view(&[]);
    assert!(empty.is_empty());
    assert_eq!(empty.peek_u8(0), None);
    assert_eq!(empty.slice_at(0, 0), Some(&[][..]));
}

#[test]
fn outbuf_never_writes_past_capacity() {
    let mut storage = [0xccu8; 8 + 16];
    let mut out = unsafe { OutBuf::new(storage.as_mut_ptr(), 8).unwrap() };
    for i in 0..8 {
        assert!(out.push(i));
    }
    assert!(!out.push(9));
    assert_eq!(out.pos(), 8);
    assert_eq!(out.remaining(), 0);
    assert_eq!(&storage[..8], &[0, 1, 2, 3, 4, 5, 6, 7]);
    assert!(storage[8..].iter().all(|&b| b == 0xcc));

    let mut storage = [0xccu8; 4 + 16];
    let mut out = unsafe { OutBuf::new(storage.as_mut_ptr(), 4).unwrap() };
    assert!(!out.extend(&[1, 2, 3, 4, 5]));
    assert_eq!(out.pos(), 0);
    assert!(storage.iter().all(|&b| b == 0xcc));
}

#[test]
fn namewriter_handles_exact_fit() {
    let mut storage = [0xccu8; 4 + 16];
    let mut out = unsafe { OutBuf::new(storage.as_mut_ptr(), 4).unwrap() };
    {
        let mut writer = NameWriter::new(&mut out);
        assert!(writer.label(b"a"));
        assert!(writer.label(b"b"));
        assert!(writer.finish());
    }
    assert_eq!(&storage[..4], b"a.b\0");
    assert!(storage[4..].iter().all(|&b| b == 0xcc));

    // "a.b" fits in 3 bytes, but the terminating NUL does not.
    let mut storage = [0xccu8; 3 + 16];
    let mut out = unsafe { OutBuf::new(storage.as_mut_ptr(), 3).unwrap() };
    {
        let mut writer = NameWriter::new(&mut out);
        assert!(writer.label(b"a"));
        assert!(writer.label(b"b"));
        assert!(!writer.finish());
    }
    assert_eq!(&storage[..3], b"a.b");
    assert!(storage[3..].iter().all(|&b| b == 0xcc));
}

#[test]
fn dns_skip_name_handles_truncation() {
    assert_eq!(dns_skip_name(&view(&[0]), 0), Some(1));
    assert_eq!(dns_skip_name(&view(&[1, b'a', 0]), 0), Some(3));
    assert_eq!(dns_skip_name(&view(&[3, b'a', b'b']), 0), None);
    assert_eq!(dns_skip_name(&view(&[0xc0, 0]), 0), Some(2));
    assert_eq!(dns_skip_name(&view(&[0xc0]), 0), None);
    assert_eq!(dns_skip_name(&view(&[1, b'a', 0]), 3), None);
}

fn expand(input: &[u8], cap: usize) -> (Option<usize>, [u8; 32]) {
    let mut output = [0xccu8; 32];
    let msg = view(input);
    let mut out = unsafe { OutBuf::new(output.as_mut_ptr(), cap).unwrap() };
    let result = dns_expand_name_at(&msg, 0, &mut out, input.len() / 2 + 1, 16);
    (result, output)
}

#[test]
fn dns_expand_name_stays_in_bounds() {
    let patterns: &[&[u8]] = &[
        &[],
        &[0],
        &[1, b'a'],
        &[1, b'a', 0],
        &[1, b'a', 1, b'b', 0],
        &[3, b'a', b'b', b'c', 1, b'd', 1, b'e', 0],
        &[0xc0, 0x00],
        &[0xc0, 0xff],
        &[0xff],
    ];
    for pattern in patterns {
        for cap in 0..=8 {
            let (_, output) = expand(pattern, cap);
            assert!(
                output[cap..].iter().all(|&b| b == 0xcc),
                "wrote past capacity {cap} for {pattern:?}"
            );
        }
    }
}

#[test]
fn dns_expand_name_short_inputs_stay_in_bounds() {
    // Exhaustive over all two-byte messages: covers terminator, label,
    // truncated label, pointer, and self-pointer combinations.
    for a in 0..=255u8 {
        for b in 0..=255u8 {
            let input = [a, b];
            for cap in 0..=4 {
                let (_, output) = expand(&input, cap);
                assert!(
                    output[cap..].iter().all(|&x| x == 0xcc),
                    "wrote past capacity {cap} for [{a:#04x}, {b:#04x}]"
                );
            }
        }
    }
}

#[test]
fn dns_expand_name_results() {
    let (result, output) = expand(&[1, b'a', 1, b'b', 0], 4);
    assert_eq!(result, Some(5));
    assert_eq!(&output[..4], b"a.b\0");

    let (result, output) = expand(&[1, b'a', 1, b'b', 0], 3);
    assert_eq!(result, None);
    assert_eq!(&output[..3], b"a.b");
    assert!(output[3..].iter().all(|&b| b == 0xcc));

    // A compression pointer that targets itself must terminate through the
    // step limit rather than loop forever.
    let (result, output) = expand(&[0xc0, 0x00], 8);
    assert_eq!(result, None);
    assert!(output.iter().all(|&b| b == 0xcc));

    // A pointer outside the message is rejected.
    let (result, output) = expand(&[0xc0, 0x40], 8);
    assert_eq!(result, None);
    assert!(output.iter().all(|&b| b == 0xcc));
}
