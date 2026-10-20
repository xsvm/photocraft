//! P30 A/B: one full-resolution Liquify update (800 px Forward Warp on 3500×2200, P30's
//! view), timed with the stroke segment excluded. `tex` fills the source with noise.
use photocraft_algo::liquify::{LiquifyField, LiquifyStroke, LiquifyTool, ProxyImage, auto_cell};
use photocraft_color::{ColorMode, PixelFormat, SampleType};
use photocraft_geom::Rect;
use photocraft_raster::Surface;
use std::time::Instant;

fn main() {
    let canvas = Rect::new(0, 0, 3500, 2200);
    let view = Rect::new(1150, 700, 2350, 1500);
    let textured = std::env::args().nth(1).as_deref() == Some("tex");
    let mut source = Surface::with_default(PixelFormat::new(ColorMode::Rgb, SampleType::U8, true), &[0.32, 0.48, 0.64, 1.0]);
    if textured {
        let n = (canvas.width() * canvas.height()) as usize;
        let mut d = Vec::with_capacity(n * 4);
        let mut s = 0x2545_f491u32;
        for i in 0..n {
            for _ in 0..3 {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                d.push((s >> 24) as f32 / 255.0);
            }
            d.push(if i % 7 == 0 { 0.5 } else { 1.0 });
        }
        source.write_region(canvas, &d);
    }
    let mut out = vec![[0u8; 4]; view.width() as usize * view.height() as usize];
    let mut scratch = Vec::new();
    let mut stroke = LiquifyStroke::new(LiquifyTool::ForwardWarp, 800.0);
    stroke.density = 100.0;
    stroke.points = vec![vec![1470.0, 1100.0, 1.0], vec![1530.0, 1100.0, 1.0]];
    let mut field = LiquifyField::new(canvas, auto_cell(canvas));
    let mut times = Vec::new();
    let mut fp = 0xcbf2_9ce4_8422_2325u64;
    for _ in 0..31 {
        let dirty = field.stroke_segment(&stroke, [1470.0, 1100.0, 1.0], [1530.0, 1100.0, 1.0]);
        let t = Instant::now();
        assert!(ProxyImage::render_full_resolution(&source, &field, view, dirty, &mut out, &mut scratch));
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        fp = out.iter().flatten().fold(fp, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3));
    }
    times.sort_by(f64::total_cmp);
    println!("textured={textured} render p50 {:.2} ms min {:.2} ms fp {fp:016x}", times[15], times[0]);
}
