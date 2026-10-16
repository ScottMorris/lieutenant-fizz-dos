// Point-light pool that selects the nearest lights to the camera each frame.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// One point light in world space (`z` lifts it off the sprite plane for Lambert shading).
#[allow(unused_imports)]
use alloc::{boxed::Box, vec, vec::Vec};
#[allow(unused_imports)]
use crate::fmath::{FloatMath, FloatMath32};
#[derive(Clone, Copy, Debug, Default)]
pub struct Light {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub r: f64,
    pub cr: f64,
    pub cg: f64,
    pub cb: f64,
    /// Squared distance to the camera, used for nearest-first selection. Negative sorts first.
    pub d: f64,
}

pub const MAX_ACTIVE: usize = 16;

/// A reusable pool of candidate lights; the nearest 16 are written to the shader uniforms.
#[derive(Debug)]
pub struct LightPool {
    pool: Vec<Light>,
    n: usize,
}

impl LightPool {
    pub fn new(cap: usize) -> Self {
        LightPool {
            pool: vec![Light::default(); cap],
            n: 0,
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

    /// Adds a light, measuring distance from the camera at (`cam_x`, `cam_y`). Returns its index.
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        &mut self,
        cam: (f64, f64),
        x: f64,
        y: f64,
        z: f64,
        r: f64,
        c: [f64; 3],
    ) -> Option<usize> {
        if self.n >= self.pool.len() {
            return None;
        }
        let i = self.n;
        self.pool[i] = Light {
            x,
            y,
            z,
            r,
            cr: c[0],
            cg: c[1],
            cb: c[2],
            d: (x - cam.0).powi(2) + (y - cam.1).powi(2),
        };
        self.n += 1;
        Some(i)
    }

    /// Forces light `i` to sort before everything else (the player's lantern).
    pub fn pin_first(&mut self, i: usize) {
        self.pool[i].d = -1.0;
    }

    /// Selection-sorts the nearest lights to the front and writes up to 16 into `pos`
    /// (x, y, z, radius per light) and `col` (r, g, b per light, scaled by `mul`).
    /// Returns the number written.
    pub fn select(&mut self, mul: f64, pos: &mut [f32], col: &mut [f32]) -> usize {
        let cnt = self.n.min(MAX_ACTIVE);
        for k in 0..cnt {
            let mut mi = k;
            for j in (k + 1)..self.n {
                if self.pool[j].d < self.pool[mi].d {
                    mi = j;
                }
            }
            self.pool.swap(k, mi);
            let o = self.pool[k];
            pos[k * 4] = o.x as f32;
            pos[k * 4 + 1] = o.y as f32;
            pos[k * 4 + 2] = o.z as f32;
            pos[k * 4 + 3] = o.r as f32;
            col[k * 3] = (o.cr * mul) as f32;
            col[k * 3 + 1] = (o.cg * mul) as f32;
            col[k * 3 + 2] = (o.cb * mul) as f32;
        }
        cnt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_first_and_pinned_lantern() {
        let mut lp = LightPool::new(64);
        for i in 0..20 {
            lp.add(
                (0.0, 0.0),
                f64::from(20 - i),
                0.0,
                1.0,
                3.0,
                [1.0, 1.0, 1.0],
            );
        }
        let lantern = lp
            .add((0.0, 0.0), 50.0, 0.0, 1.0, 3.0, [1.0, 0.0, 0.0])
            .unwrap();
        lp.pin_first(lantern);
        let (mut pos, mut col) = ([0.0f32; 64], [0.0f32; 48]);
        let n = lp.select(0.5, &mut pos, &mut col);
        assert_eq!(n, 16);
        assert_eq!(pos[0], 50.0);
        assert_eq!(col[0], 0.5);
        assert_eq!(pos[4], 1.0);
        assert_eq!(pos[8], 2.0);
    }
}
