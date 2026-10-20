//! P32 bisect: perf_scenarios' "brush release" row without the canvas (its refresh is of an
//! empty rect): offer the live 600-point stroke, then commit it. Prints the min and median of
//! each phase; exits 1 when the release is slow (bad), 0 when fast (good).
use photocraft_engine::{Session, brush_cmds::LiveStroke, paint::StrokePoint};
use serde_json::json;
use std::time::Instant;

fn main() {
    let (w, h) = (6000u32, 4000u32);
    let mut s = Session::new();
    s.execute("file.new", json!({"width": w, "height": h, "background": "white"})).unwrap();
    s.execute("layer.new.layer", json!({"name": "paint"})).unwrap();
    let (mut offer, mut commit) = (Vec::new(), Vec::new());
    for i in 0..7 {
        let (fw, fh) = (f64::from(w), f64::from(h));
        let pts: Vec<[f64; 3]> = (0..600)
            .map(|k| {
                let t = k as f64 / 599.0;
                [fw * (0.1 + 0.8 * t), fh * (0.5 + 0.35 * (t * 12.0 + i as f64).sin()), 0.6 + 0.4 * (t * 7.0).sin().abs()]
            })
            .collect();
        let mut p = json!({"points": [pts[0]], "size": 200, "hardness": 0.0, "color": "#c03020", "smoothing": 0.0});
        let mut live = LiveStroke::begin(&s, &p).unwrap();
        for c in pts[1..].chunks(4) {
            let sp: Vec<StrokePoint> = c.iter().map(|q| StrokePoint::new(q[0], q[1], q[2] as f32)).collect();
            live.push(&sp).unwrap();
        }
        p["points"] = json!(pts);
        p["seed"] = json!(live.seed);
        let t = Instant::now();
        s.offer_live_stroke(live).unwrap();
        let t1 = Instant::now();
        let r = s.execute("paint.stroke", p).unwrap();
        let t2 = Instant::now();
        s.clear_live_stroke();
        assert_eq!(r.get("fromLive").and_then(|v| v.as_bool()), Some(true), "the release rendered the stroke again");
        offer.push((t1 - t).as_secs_f64() * 1000.0);
        commit.push((t2 - t1).as_secs_f64() * 1000.0);
    }
    offer.sort_by(f64::total_cmp);
    commit.sort_by(f64::total_cmp);
    let total = offer[0] + commit[0];
    println!("offer min {:.2} p50 {:.2} | commit min {:.2} p50 {:.2} | total min {total:.2} ms", offer[0], offer[3], commit[0], commit[3]);
    std::process::exit(i32::from(total > 1.5));
}
