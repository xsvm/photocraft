//! Fork-only analysis: releasing a live Clone Stamp / History Brush stroke, rendered again (as on
//! `main`) vs taking the live pixels (offer_live_retouch), and whether both give the same pixels.
use photocraft_engine::Session;
use photocraft_engine::retouch_cmds::LiveRetouch;
use photocraft_paint::StrokePoint;
use serde_json::{Value, json};
use std::time::Instant;

fn session(w: u32, h: u32, depth: u64) -> Session {
    let mut s = Session::new();
    s.execute("file.new", json!({"width": w, "height": h, "depth": depth, "mode": "rgb"})).unwrap();
    s.execute("filter.addNoise", json!({"amount": 30})).ok();
    s.execute("paint.stroke", json!({"points": [[0, 2000], [6000, 2100]], "size": 400, "hardness": 0.5, "color": "#c03030"})).unwrap();
    s
}

fn main() {
    let (w, h) = (6000u32, 4000u32);
    let pts: Vec<[f64; 2]> = (0..1200)
        .map(|i| {
            let t = i as f64 / 1200.0;
            [500.0 + 5000.0 * t + 150.0 * (t * 60.0).sin(), 2000.0 + 1200.0 * (t * 7.0).sin()]
        })
        .collect();
    println!("{w}x{h}, {} points, size 120", pts.len());
    for depth in [8u64, 16, 32] {
        for (cmd, extra) in [("paint.cloneStamp", json!({"source": [300, 3500]})), ("paint.historyBrush", json!({"state": 0}))] {
            let mut p = json!({"size": 120, "hardness": 50, "opacity": 80});
            for (k, v) in extra.as_object().unwrap() {
                p[k] = v.clone();
            }
            let with = |v: &[[f64; 2]]| {
                let mut q: Value = p.clone();
                q["points"] = json!(v);
                q
            };
            let mut old = session(w, h, depth);
            let t = Instant::now();
            old.execute(cmd, with(&pts)).unwrap();
            let rerender = t.elapsed().as_secs_f64() * 1000.0;

            let mut s = session(w, h, depth);
            let mut live = LiveRetouch::begin(&s, cmd, &with(&pts[..1])).unwrap();
            let mut i = 1;
            while i < pts.len() {
                let n = (1 + i % 3).min(pts.len() - i);
                let chunk: Vec<_> = pts[i..i + n].iter().map(|q| StrokePoint::new(q[0], q[1], 1.0)).collect();
                live.push(&chunk).unwrap();
                i += n;
            }
            let t = Instant::now();
            s.offer_live_retouch(live).unwrap();
            let r = s.execute(cmd, with(&pts)).unwrap();
            s.clear_live_stroke();
            let taken = t.elapsed().as_secs_f64() * 1000.0;

            let (a, b) = (s.active().unwrap(), old.active().unwrap());
            let bounds = a.doc.bounds();
            let x = a.doc.layer(a.active_layer.unwrap()).unwrap().surface().unwrap().read_region(bounds);
            let y = b.doc.layer(b.active_layer.unwrap()).unwrap().surface().unwrap().read_region(bounds);
            let differing = x.iter().zip(&y).filter(|(u, v)| u.to_bits() != v.to_bits()).count();
            println!(
                "{depth:2}-bit {cmd:18} release: rendered again {rerender:7.1} ms, live pixels {taken:6.2} ms (fromLive={}) | differing values {differing} of {}",
                r["fromLive"],
                x.len()
            );
        }
    }
}
