//! Fork-only analysis: per-event cost of a long live stroke, symmetry off / vertical / dual.
use photocraft_engine::{Session, brush_cmds::LiveStroke};
use photocraft_paint::StrokePoint;
use serde_json::json;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for size in [30, 120] {
        for mode in ["off", "vertical", "dual"] {
            let mut s = Session::new();
            s.execute("file.new", json!({"width":6000,"height":4000,"depth":8,"mode":"rgb"}))?;
            if mode != "off" {
                s.execute("paint.setSymmetry", json!({"mode":mode}))?;
            }
            let params = json!({"points":[[400,600]],"size":size,"hardness":0.8,"opacity":0.5,"smoothing":0.0,"seed":7});
            let mut live = LiveStroke::begin(&s, &params)?;
            let n = 1200;
            let mut t = Vec::with_capacity(n);
            for i in 1..=n {
                let f = i as f64;
                let start = Instant::now();
                live.push(&[StrokePoint::new(400.0 + f * 2.0, 600.0 + f * 1.2 + 80.0 * (f / 25.0).sin(), 1.0)])?;
                t.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            std::hint::black_box(&live.doc);
            let avg = |a: &[f64]| a.iter().sum::<f64>() / a.len() as f64;
            let total: f64 = t.iter().sum();
            println!(
                "size {size:3} {mode:8}: first100 avg {:7.3} ms | last100 avg {:8.3} ms | max {:8.3} ms | total {:9.1} ms / {n} events",
                avg(&t[..100]),
                avg(&t[n - 100..]),
                t.iter().cloned().fold(0.0, f64::max),
                total
            );
        }
    }
    Ok(())
}
