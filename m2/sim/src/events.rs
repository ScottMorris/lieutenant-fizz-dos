// Flat event queue the simulation fills and the shell reads from linear memory.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// A flat event: a kind code plus five numeric arguments. The shell reads these straight out
/// of linear memory (stride 6 floats), so there is no serialisation.
#[allow(unused_imports)]
use alloc::{boxed::Box, vec, vec::Vec};
#[allow(unused_imports)]
use crate::fmath::{FloatMath, FloatMath32};
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Event {
    pub kind: f32,
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
}

pub const EVENT_STRIDE: usize = 6;

/// Fixed-capacity event queue backed by one flat `f32` buffer; the oldest events are dropped
/// on overflow rather than allocating.
#[derive(Debug)]
pub struct EventQueue {
    buf: Vec<f32>,
    count: usize,
    cap: usize,
}

impl EventQueue {
    pub fn new(cap: usize) -> Self {
        EventQueue {
            buf: vec![0.0; cap * EVENT_STRIDE],
            count: 0,
            cap,
        }
    }

    pub fn push(&mut self, ev: Event) {
        if self.count >= self.cap {
            return;
        }
        let o = self.count * EVENT_STRIDE;
        self.buf[o..o + EVENT_STRIDE].copy_from_slice(&[ev.kind, ev.a, ev.b, ev.c, ev.d, ev.e]);
        self.count += 1;
    }

    pub fn emit(&mut self, kind: u32, a: f64, b: f64, c: f64) {
        self.push(Event {
            kind: kind as f32,
            a: a as f32,
            b: b as f32,
            c: c as f32,
            ..Default::default()
        });
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn clear(&mut self) {
        self.count = 0;
    }

    pub fn ptr(&self) -> *const f32 {
        self.buf.as_ptr()
    }

    pub fn get(&self, i: usize) -> Option<Event> {
        if i >= self.count {
            return None;
        }
        let o = i * EVENT_STRIDE;
        let s = &self.buf[o..o + EVENT_STRIDE];
        Some(Event {
            kind: s[0],
            a: s[1],
            b: s[2],
            c: s[3],
            d: s[4],
            e: s[5],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_get_clear() {
        let mut q = EventQueue::new(2);
        q.emit(1, 2.0, 3.0, 4.0);
        q.emit(5, 0.0, 0.0, 0.0);
        q.emit(9, 0.0, 0.0, 0.0); // dropped
        assert_eq!(q.len(), 2);
        assert_eq!(q.get(0).unwrap().b, 3.0);
        assert!(q.get(2).is_none());
        q.clear();
        assert!(q.is_empty());
    }
}
