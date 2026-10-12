//! Fork-only analysis: lasso polygon fill and select.lasso on a 24 MP document.
use photocraft_algo::selection::polygon;
use photocraft_engine::Session;
use photocraft_geom::Rect;
use serde_json::json;
use std::time::Instant;

fn best<T>(n: usize, mut f: impl FnMut() -> T) -> f64 {
    (0..n)
        .map(|_| {
            let t = Instant::now();
            std::hint::black_box(f());
            t.elapsed().as_secs_f64() * 1000.0
        })
        .fold(f64::INFINITY, f64::min)
}

fn wobbly(count: usize, cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<(f32, f32)> {
    (0..count)
        .map(|i| {
            let a = i as f32 / count as f32 * std::f32::consts::TAU;
            (cx + rx * a.cos() * (1.0 + 0.05 * (a * 37.0).sin()), cy + ry * a.sin())
        })
        .collect()
}

fn main() {
    let (w, h) = (6000u32, 4000u32);
    let area = Rect::new(0, 0, w as i32, h as i32);
    let mut s = Session::new();
    s.execute("file.new", json!({"width":w,"height":h,"depth":8,"mode":"rgb"})).unwrap();
    println!("24 MP ({w}x{h}), best of 3");
    for (label, rx, ry) in [("large", 2700.0, 1800.0), ("small", 150.0, 100.0)] {
        for count in [100usize, 1_000, 10_000, 100_000] {
            let pts = wobbly(count, 3000.0, 2000.0, rx, ry);
            let fill = best(3, || polygon(&pts, area, true));
            let json_pts: Vec<[f32; 2]> = pts.iter().map(|p| [p.0, p.1]).collect();
            let cmd = best(3, || s.execute("select.lasso", json!({"points": json_pts})).unwrap());
            println!("{label:5} {count:6} points: polygon {fill:8.1} ms, select.lasso {cmd:8.1} ms");
        }
    }
}
