//! The deep-space background, as a **cubemap** on the cockpit camera (DL-018,
//! supersedes DL-006's point-starfield).
//!
//! A cubemap sits at infinity: turning the ship reveals different sky (and a
//! band/galaxy in some directions) while translation correctly shows *no*
//! parallax — the faithful "infinitely distant stars" feel, and more physically
//! correct than the old translation-following shell.
//!
//! This module generates a **placeholder** cubemap at runtime: deterministic
//! scattered stars plus a faint, tilted band for directional structure. It stands
//! in for a real astronomical image — swapping it in is a one-line change (load a
//! cubemap asset instead of calling [`placeholder_cubemap`]). Asset-free for now,
//! so builds stay light until the real image is chosen.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

/// Pixels per cube face (six faces → a `FACE_SIZE² × 6` image).
const FACE_SIZE: u32 = 256;
/// Fraction of pixels that become a star.
const STAR_CHANCE: f32 = 0.004;
/// Half-width of the faint band, as `|dir · band_normal|`.
const BAND_WIDTH: f32 = 0.33;

/// Build the placeholder star cubemap, add it to the image assets, and return its
/// handle. Replace this call with a loaded cubemap asset for a real sky.
pub fn placeholder_cubemap(images: &mut Assets<Image>) -> Handle<Image> {
    let mut data = Vec::with_capacity((FACE_SIZE * FACE_SIZE * 6 * 4) as usize);

    // A faint, tilted "Milky Way" band gives the sky directional structure.
    let band_normal = Vec3::new(0.25, 1.0, 0.35).normalize();

    for face in 0..6u32 {
        for y in 0..FACE_SIZE {
            for x in 0..FACE_SIZE {
                data.extend_from_slice(&pixel(face, x, y, band_normal));
            }
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: FACE_SIZE,
            height: FACE_SIZE,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // View the six array layers as cube faces.
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });

    images.add(image)
}

/// One RGBA pixel: a star, or faint space brightened near the band.
fn pixel(face: u32, x: u32, y: u32, band_normal: Vec3) -> [u8; 4] {
    if hash01(mix(face, x, y)) < STAR_CHANCE {
        // A star — vary brightness, and tint it faintly blue-white (red eased
        // down, blue full). `b ∈ [150, 255)`, so the scaled channels never overflow.
        let b = 150.0 + 105.0 * hash01(mix(face ^ 0x5bd1, x, y));
        return [(b * 0.92) as u8, (b * 0.96) as u8, b as u8, 255];
    }
    // Faint space, brightened toward the band plane (|dir · n| → 0).
    let dir = face_direction(face, x, y);
    let band = (1.0 - dir.dot(band_normal).abs() / BAND_WIDTH).clamp(0.0, 1.0);
    let glow = (band * band * 16.0) as u8;
    [2 + glow, 2 + glow, 6 + glow, 255]
}

/// The unit world-space direction a cube face's pixel looks toward (wgpu face
/// order: +X, -X, +Y, -Y, +Z, -Z).
fn face_direction(face: u32, x: u32, y: u32) -> Vec3 {
    let u = (x as f32 + 0.5) / FACE_SIZE as f32 * 2.0 - 1.0;
    let v = (y as f32 + 0.5) / FACE_SIZE as f32 * 2.0 - 1.0;
    let dir = match face {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    };
    dir.normalize()
}

/// Mix a face index and pixel coordinates into a hash seed.
fn mix(face: u32, x: u32, y: u32) -> u32 {
    face.wrapping_mul(0x9E37_79B1) ^ x.wrapping_mul(0x85EB_CA77) ^ y.wrapping_mul(0xC2B2_AE3D)
}

/// Deterministic integer hash → `[0.0, 1.0)`.
fn hash01(mut h: u32) -> f32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::{FACE_SIZE, face_direction, hash01};
    use bevy::prelude::Vec3;

    /// The canonical OpenGL/wgpu cube projection: a unit direction → its
    /// `(face, texel x, y)`. Derived independently from the spec — *not* from
    /// [`face_direction`] — so a round-trip through both catches any per-face
    /// `u`/`v` sign error (the center-only test below cannot).
    fn project_to_cube(d: Vec3) -> (u32, u32, u32) {
        let (ax, ay, az) = (d.x.abs(), d.y.abs(), d.z.abs());
        let (face, sc, tc, ma) = if ax >= ay && ax >= az {
            if d.x > 0.0 {
                (0u32, -d.z, -d.y, ax)
            } else {
                (1, d.z, -d.y, ax)
            }
        } else if ay >= az {
            if d.y > 0.0 {
                (2, d.x, d.z, ay)
            } else {
                (3, d.x, -d.z, ay)
            }
        } else if d.z > 0.0 {
            (4, d.x, -d.y, az)
        } else {
            (5, -d.x, -d.y, az)
        };
        let u = (sc / ma + 1.0) * 0.5;
        let v = (tc / ma + 1.0) * 0.5;
        let x = ((u * FACE_SIZE as f32) as u32).min(FACE_SIZE - 1);
        let y = ((v * FACE_SIZE as f32) as u32).min(FACE_SIZE - 1);
        (face, x, y)
    }

    #[test]
    fn the_cubemap_is_continuous_across_faces() {
        // Round-trip a grid of directions: direction → (face, texel) via the
        // canonical projection → `face_direction` → back to (nearly) the same
        // direction. A flipped `u`/`v` sign on any face breaks this (and would show
        // as a seam) — the kind of bug the center-only test and a smoke launch miss.
        let n: u32 = 17;
        let to_axis = |t: u32| t as f32 / (n - 1) as f32 * 2.0 - 1.0;
        let mut max_err = 0.0_f32;
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    let d = Vec3::new(to_axis(i), to_axis(j), to_axis(k));
                    if d.length_squared() < 1.0e-6 {
                        continue;
                    }
                    let d = d.normalize();
                    let (face, x, y) = project_to_cube(d);
                    max_err = max_err.max(1.0 - d.dot(face_direction(face, x, y)));
                }
            }
        }
        assert!(
            max_err < 0.01,
            "cubemap round-trip error {max_err} — a face u/v sign is likely flipped"
        );
    }

    #[test]
    fn hash01_stays_in_unit_interval() {
        for i in 0..4000u32 {
            let h = hash01(i);
            assert!((0.0..1.0).contains(&h), "hash01({i}) = {h} out of range");
        }
    }

    #[test]
    fn face_directions_are_unit_and_point_along_their_axis() {
        let c = FACE_SIZE / 2;
        for face in 0..6u32 {
            let d = face_direction(face, c, c);
            assert!(
                (d.length() - 1.0).abs() < 1e-5,
                "face {face} not unit: {d:?}"
            );
        }
        // The +X and -X faces look in opposite directions.
        assert!(face_direction(0, c, c).x > 0.9, "+X face looks +X");
        assert!(face_direction(1, c, c).x < -0.9, "-X face looks -X");
    }
}
