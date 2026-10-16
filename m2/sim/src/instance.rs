// Instance buffer of sprite draw records shared with the renderer.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[allow(unused_imports)]
use alloc::{boxed::Box, vec, vec::Vec};
#[allow(unused_imports)]
use crate::fmath::{FloatMath, FloatMath32};
use crate::STRIDE;

/// Atlas rectangle of a sprite (UV origin and size) plus its size in logical pixels.
#[derive(Clone, Copy, Debug, Default)]
pub struct SpriteRect {
    pub u: f32,
    pub v: f32,
    pub uw: f32,
    pub vh: f32,
    pub w: f32,
    pub h: f32,
}

/// Per-instance draw options. Defaults draw an opaque, untinted, lit sprite at 1x.
#[derive(Clone, Copy, Debug)]
pub struct PushOpts {
    pub scale: f32,
    pub sx: f32,
    pub sy: f32,
    pub flip: bool,
    pub rot: f32,
    pub tint: u32,
    pub alpha: f32,
    /// Emissive: bypasses lighting.
    pub emissive: bool,
    /// Actor: lit with a minimum brightness so characters never vanish in the dark.
    pub actor: bool,
}

impl Default for PushOpts {
    fn default() -> Self {
        PushOpts {
            scale: 1.0,
            sx: 1.0,
            sy: 1.0,
            flip: false,
            rot: 0.0,
            tint: 0xff_ffff,
            alpha: 1.0,
            emissive: false,
            actor: false,
        }
    }
}

impl PushOpts {
    pub fn em() -> Self {
        PushOpts {
            emissive: true,
            ..Default::default()
        }
    }
    pub fn tint(tint: u32) -> Self {
        PushOpts {
            tint,
            ..Default::default()
        }
    }
}

/// The shared instance buffer: `capacity` instances of 20 floats, written front to back each
/// frame (painter's order) and drawn by one instanced call. Layout per instance:
/// 0..4 = matrix row 0 (w = UV width), 4..8 = row 1 (w = UV height), 8..12 = row 2,
/// 12..16 = translation, 16..20 = (u, v, packed tint RGB, alpha + 2*emissive + 4*actor).
#[derive(Debug)]
pub struct InstanceBuffer {
    buf: Vec<f32>,
    n: usize,
    cap: usize,
}

impl InstanceBuffer {
    pub fn new(capacity: usize) -> Self {
        let mut buf = vec![0.0; capacity * STRIDE];
        for i in 0..capacity {
            buf[i * STRIDE + 10] = 1.0;
            buf[i * STRIDE + 15] = 1.0;
        }
        InstanceBuffer {
            buf,
            n: 0,
            cap: capacity,
        }
    }

    pub fn clear(&mut self) {
        self.n = 0;
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn ptr(&self) -> *const f32 {
        self.buf.as_ptr()
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.buf[..self.n * STRIDE]
    }

    /// Appends one sprite centred at (`x`, `y`). Silently drops instances past capacity.
    pub fn push(&mut self, x: f64, y: f64, sp: &SpriteRect, o: &PushOpts) {
        if self.n >= self.cap {
            return;
        }
        let b = &mut self.buf;
        let i = self.n * STRIDE;
        let sx = sp.w / 16.0 * o.scale * if o.flip { -1.0 } else { 1.0 } * o.sx;
        let sy = sp.h / 16.0 * o.scale * o.sy;
        if o.rot != 0.0 {
            let (sn, cs) = o.rot.sin_cos();
            b[i] = cs * sx;
            b[i + 1] = sn * sx;
            b[i + 4] = -sn * sy;
            b[i + 5] = cs * sy;
        } else {
            b[i] = sx;
            b[i + 1] = 0.0;
            b[i + 4] = 0.0;
            b[i + 5] = sy;
        }
        b[i + 3] = sp.uw;
        b[i + 7] = sp.vh;
        b[i + 12] = x as f32;
        b[i + 13] = y as f32;
        b[i + 16] = sp.u;
        b[i + 17] = sp.v;
        b[i + 18] = o.tint as f32;
        b[i + 19] = o.alpha + if o.emissive { 2.0 } else { 0.0 } + if o.actor { 4.0 } else { 0.0 };
        self.n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_packs_matrix_uv_and_flags() {
        let mut ib = InstanceBuffer::new(4);
        let sp = SpriteRect {
            u: 0.25,
            v: 0.5,
            uw: 0.03,
            vh: 0.04,
            w: 16.0,
            h: 32.0,
        };
        ib.push(
            3.0,
            4.0,
            &sp,
            &PushOpts {
                flip: true,
                emissive: true,
                actor: true,
                ..Default::default()
            },
        );
        let s = ib.as_slice();
        assert_eq!(s.len(), STRIDE);
        assert_eq!(s[0], -1.0);
        assert_eq!(s[5], 2.0);
        assert_eq!(s[3], 0.03);
        assert_eq!(s[7], 0.04);
        assert_eq!((s[12], s[13]), (3.0, 4.0));
        assert_eq!((s[16], s[17]), (0.25, 0.5));
        assert_eq!(s[19], 7.0);
    }

    #[test]
    fn drops_past_capacity() {
        let mut ib = InstanceBuffer::new(1);
        let sp = SpriteRect::default();
        ib.push(0.0, 0.0, &sp, &PushOpts::default());
        ib.push(0.0, 0.0, &sp, &PushOpts::default());
        assert_eq!(ib.len(), 1);
    }
}
