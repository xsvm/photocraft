//! Fork-only analysis: does a live retouching stroke show exactly what its release commits,
//! and how long does the release take?
use photocraft_engine::Session;
use photocraft_engine::retouch_cmds::{LiveDab, LiveRetouch};
use photocraft_paint::StrokePoint;
use serde_json::{Value, json};
use std::time::Instant;

fn texture(x: i32, y: i32) -> [f32; 4] {
    let n = ((x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663)) as u32 % 1000) as f32 / 1000.0 * 0.2 - 0.1;
    [0.2 + 0.0004 * x as f32 + n, 0.3 + 0.0005 * y as f32 + n, 0.45 + n + 0.1 * ((x / 37 + y / 23) % 3) as f32, 1.0]
}

fn session(w: u32, h: u32, depth: u64) -> Session {
    let mut s = Session::new();
    s.execute("file.new", json!({"width": w, "height": h, "depth": depth, "mode": "rgb"})).unwrap();
    s.edit("setup", |doc, active| {
        let b = doc.bounds();
        let surf = doc.layer_mut(active.unwrap()).unwrap().surface_mut().unwrap();
        let fmt = surf.format();
        let mut data = Vec::new();
        for y in b.y0..b.y1 {
            for x in b.x0..b.x1 {
                data.extend(photocraft_raster::from_rgba(&fmt, texture(x, y)));
            }
        }
        surf.write_region(b, &data);
        Ok(())
    })
    .unwrap();
    // A second state for the History Brush to paint back from.
    s.execute("paint.stroke", json!({"points": [[0, 500], [3000, 520]], "size": 300, "hardness": 0.5, "color": "#c03030"})).unwrap();
    s
}

/// A stroke that doubles back over itself, as pointer events would deliver it.
fn stroke() -> Vec<[f64; 2]> {
    (0..600)
        .map(|i| {
            let t = i as f64 / 600.0;
            [300.0 + 2200.0 * t + 120.0 * (t * 40.0).sin(), 600.0 + 350.0 * (t * 9.0).sin()]
        })
        .collect()
}

fn with_points(base: &Value, pts: &[[f64; 2]]) -> Value {
    let mut p = base.clone();
    p["points"] = json!(pts);
    p
}

enum Live {
    R(LiveRetouch),
    D(LiveDab),
}

impl Live {
    fn push(&mut self, pts: &[StrokePoint]) {
        match self {
            Live::R(l) => l.push(pts).map(|_| ()),
            Live::D(l) => l.push(pts).map(|_| ()),
        }
        .unwrap()
    }
    fn doc(&self) -> &photocraft_doc::Document {
        match self {
            Live::R(l) => &*l.doc,
            Live::D(l) => &*l.doc,
        }
    }
}

fn main() {
    let (w, h) = (3000u32, 1400u32);
    let pts = stroke();
    let base = json!({"size": 90, "hardness": 50, "opacity": 80});
    let tools: [(&str, Value); 8] = [
        ("paint.cloneStamp", json!({"source": [200, 1100]})),
        ("paint.historyBrush", json!({"state": 0})),
        ("paint.dodge", json!({"range": "midtones", "exposure": 50})),
        ("paint.burn", json!({"range": "shadows", "exposure": 50})),
        ("paint.sponge", json!({"mode": "saturate", "flow": 50})),
        ("paint.blur", json!({"strength": 50})),
        ("paint.sharpen", json!({"strength": 50})),
        ("paint.smudge", json!({"strength": 50})),
    ];
    println!("{w}x{h}, {} points, live fed 1-3 points per push; diff over the whole layer", pts.len());
    for depth in [8u64, 16, 32] {
        for (cmd, extra) in &tools {
            let mut p = base.clone();
            for (k, v) in extra.as_object().unwrap() {
                p[k] = v.clone();
            }
            let mut s = session(w, h, depth);
            let first = with_points(&p, &pts[..1]);
            let t = Instant::now();
            let mut live = if matches!(*cmd, "paint.cloneStamp" | "paint.historyBrush") {
                Live::R(LiveRetouch::begin(&s, cmd, &first).unwrap())
            } else {
                Live::D(LiveDab::begin(&s, cmd, &first).unwrap())
            };
            let mut i = 1;
            let mut k = 0;
            while i < pts.len() {
                let n = (1 + k % 3).min(pts.len() - i);
                let chunk: Vec<_> = pts[i..i + n].iter().map(|q| StrokePoint::new(q[0], q[1], 1.0)).collect();
                live.push(&chunk);
                i += n;
                k += 1;
            }
            let live_ms = t.elapsed().as_secs_f64() * 1000.0;
            let t = Instant::now();
            s.execute(cmd, with_points(&p, &pts)).unwrap();
            let commit_ms = t.elapsed().as_secs_f64() * 1000.0;
            let st = s.active().unwrap();
            let id = st.active_layer.unwrap();
            let r = st.doc.bounds();
            let a = live.doc().layer(id).unwrap().surface().unwrap().read_region(r);
            let b = st.doc.layer(id).unwrap().surface().unwrap().read_region(r);
            let (mut n, mut worst) = (0usize, 0.0f32);
            for (x, y) in a.iter().zip(&b) {
                if x.to_bits() != y.to_bits() {
                    n += 1;
                    worst = worst.max((x - y).abs());
                }
            }
            println!(
                "{depth:2}-bit {cmd:18} live total {live_ms:7.0} ms, release {commit_ms:6.0} ms | differing values {n:8} of {} , max diff {worst:.6} ({:.2} 8-bit levels)",
                a.len(),
                worst * 255.0
            );
        }
    }
}
