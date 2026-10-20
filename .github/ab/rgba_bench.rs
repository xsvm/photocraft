//! A/B: `Surface::read_rgba_into` over a 2048×2048 region (64 tiles) per format, min of 9
//! reads, plus an FNV fingerprint over every output bit (must match between sides).
use photocraft_color::{ColorMode, PixelFormat, SampleType};
use photocraft_geom::Rect;
use photocraft_raster::Surface;
use std::time::Instant;

fn main() {
    let r = Rect::new(0, 0, 2048, 2048);
    for (name, fmt) in [
        ("RGBA8", PixelFormat::RGBA8),
        ("RGB8", PixelFormat::new(ColorMode::Rgb, SampleType::U8, false)),
        ("RGBA16", PixelFormat::RGBA16),
        ("RGB16", PixelFormat::new(ColorMode::Rgb, SampleType::U16, false)),
        ("RGBA32F", PixelFormat::RGBA32F),
        ("GRAY16", PixelFormat::new(ColorMode::Grayscale, SampleType::U16, false)),
        ("GRAYA32F", PixelFormat::new(ColorMode::Grayscale, SampleType::F32, true)),
        ("LAB16", PixelFormat::new(ColorMode::Lab, SampleType::U16, true)),
    ] {
        let n = fmt.channels();
        let mut s = Surface::new(fmt);
        let mut st = 0x2545_f491u32;
        let mut data = Vec::with_capacity(r.width() as usize * r.height() as usize * n);
        for _ in 0..r.width() as usize * r.height() as usize * n {
            st ^= st << 13;
            st ^= st >> 17;
            st ^= st << 5;
            data.push((st >> 8) as f32 / 16_777_216.0);
        }
        s.write_region(r, &data);
        let mut out = vec![[0.0f32; 4]; r.width() as usize * r.height() as usize];
        let mut best = f64::MAX;
        for _ in 0..9 {
            let t = Instant::now();
            s.read_rgba_into(r, &mut out);
            best = best.min(t.elapsed().as_secs_f64() * 1000.0);
        }
        let fp = out.iter().flatten().fold(0xcbf2_9ce4_8422_2325u64, |h, v| (h ^ u64::from(v.to_bits())).wrapping_mul(0x100_0000_01b3));
        println!("{name:8} min {best:7.2} ms  fp {fp:016x}");
    }
}
