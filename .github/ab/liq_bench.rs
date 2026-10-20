//! P30 A/B: one full-resolution Liquify update (800 px Forward Warp on 3500×2200, P30's
//! view): a fresh field, one stroke segment and the render, timed like perf_scenarios. `tex` fills the source with noise.
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
    let base = LiquifyField::new(canvas, auto_cell(canvas));
    let (mut times, mut segs) = (Vec::new(), Vec::new());
    let mut fp = 0xcbf2_9ce4_8422_2325u64;
    let hash = |fp: &mut u64, out: &[[u8; 4]]| *fp = out.iter().flatten().fold(*fp, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3));
    // P30's update as perf_scenarios times it: a fresh field, one segment, then the render.
    for _ in 0..31 {
        let mut field = base.clone();
        let t = Instant::now();
        let dirty = field.stroke_segment(&stroke, [1470.0, 1100.0, 1.0], [1530.0, 1100.0, 1.0]);
        segs.push(t.elapsed().as_secs_f64() * 1000.0);
        assert!(ProxyImage::render_full_resolution(&source, &field, view, dirty, &mut out, &mut scratch));
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        hash(&mut fp, &out);
    }
    // Every brush tool, small and large brushes, a few segments each, rendered whole.
    let small = Rect::new(0, 0, 900, 700);
    let mut whole = vec![[0u8; 4]; (small.width() * small.height()) as usize];
    let mut tools_fp = 0xcbf2_9ce4_8422_2325u64;
    for tool in [
        LiquifyTool::ForwardWarp,
        LiquifyTool::Reconstruct,
        LiquifyTool::Smooth,
        LiquifyTool::TwirlCw,
        LiquifyTool::TwirlCcw,
        LiquifyTool::Pucker,
        LiquifyTool::Bloat,
        LiquifyTool::PushLeft,
        LiquifyTool::Freeze,
        LiquifyTool::Thaw,
    ] {
        for size in [40.0, 600.0] {
            let mut f = LiquifyField::new(small, 2.0);
            let mut warm = LiquifyStroke::new(LiquifyTool::ForwardWarp, 500.0);
            warm.density = 60.0;
            f.stroke_begin(&warm, [300.0, 300.0, 1.0]);
            f.stroke_segment(&warm, [300.0, 300.0, 1.0], [520.0, 380.0, 0.8]);
            let mut st = LiquifyStroke::new(tool, size);
            st.density = 50.0;
            st.rate = 80.0;
            let pts = [[350.0, 320.0, 1.0], [410.0, 340.0, 0.9], [470.0, 300.0, 0.7], [520.0, 410.0, 1.0], [600.0, 450.0, 0.5]];
            f.stroke_begin(&st, pts[0]);
            for w in pts.windows(2) {
                f.stroke_segment(&st, w[0], w[1]);
            }
            assert!(ProxyImage::render_full_resolution(&source, &f, small, small, &mut whole, &mut scratch));
            hash(&mut tools_fp, &whole);
        }
    }
    segs.sort_by(f64::total_cmp);
    println!("textured={textured} segment p50 {:.2} ms | tools fp {tools_fp:016x}", segs[15]);
    times.sort_by(f64::total_cmp);
    println!("textured={textured} update p50 {:.2} ms min {:.2} ms fp {fp:016x}", times[15], times[0]);
}
