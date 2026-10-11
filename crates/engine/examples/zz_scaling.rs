//! Fork-only analysis: does an interactive operation get slower as the state it works on grows?
use photocraft_engine::retouch_cmds::{LiveDab, LiveRetouch};
use photocraft_engine::{Session, brush_cmds::LiveStroke};
use photocraft_paint::StrokePoint;
use serde_json::{Value, json};
use std::time::Instant;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn avg(a: &[f64]) -> f64 {
    if a.is_empty() { 0.0 } else { a.iter().sum::<f64>() / a.len() as f64 }
}

fn run(s: &mut Session, cmd: &str, p: Value) -> f64 {
    let t = Instant::now();
    if let Err(e) = s.execute(cmd, p) {
        println!("    ! {cmd}: {e}");
    }
    ms(t)
}

fn doc(w: u32, h: u32) -> Session {
    let mut s = Session::new();
    s.execute("file.new", json!({"width":w,"height":h,"depth":8,"mode":"rgb"})).unwrap();
    s.execute("layer.new.layer", json!({})).unwrap();
    s.execute("edit.fill", json!({"color":"#6a8caf"})).unwrap();
    for i in 0..30 {
        let y = 100.0 + i as f64 * 120.0;
        let c = if i % 2 == 0 { "#202020" } else { "#f0e0c0" };
        let _ = s.execute("paint.stroke", json!({"points":[[50.0,y],[w as f64 - 50.0, y + 60.0]],"size":40,"color":c}));
    }
    s
}

fn path(n: usize) -> Vec<[f64; 2]> {
    (0..n)
        .map(|i| {
            let f = i as f64;
            [400.0 + f * 4.0, 600.0 + f * 2.0 + 80.0 * (f / 25.0).sin()]
        })
        .collect()
}

enum Live {
    Brush(Box<LiveStroke>),
    Retouch(LiveRetouch),
    Dab(LiveDab),
}

fn main() {
    let n = 1200;
    let pts = path(n);
    let sp = |q: &[f64; 2]| StrokePoint::new(q[0], q[1], 1.0);

    println!("== A. live tools: one stroke of {n} events on 6000x4000");
    let tools: Vec<(&str, &str, Value)> = vec![
        ("brush (reference)", "paint.stroke", json!({"size":40,"smoothing":0.0,"seed":7})),
        ("cloneStamp", "paint.cloneStamp", json!({"size":40,"offset":[-300,0]})),
        ("historyBrush", "paint.historyBrush", json!({"size":40})),
        ("smudge", "paint.smudge", json!({"size":40})),
        ("blur", "paint.blur", json!({"size":40})),
        ("dodge", "paint.dodge", json!({"size":40})),
        ("sponge", "paint.sponge", json!({"size":40})),
    ];
    for (name, cmd, base) in &tools {
        let mut s = doc(6000, 4000);
        let mut p = base.clone();
        p["points"] = json!([pts[0]]);
        let t = Instant::now();
        let live = match *cmd {
            "paint.stroke" => LiveStroke::begin(&s, &p).map(|l| Live::Brush(Box::new(l))),
            "paint.cloneStamp" | "paint.historyBrush" => LiveRetouch::begin(&s, cmd, &p).map(Live::Retouch),
            _ => LiveDab::begin(&s, cmd, &p).map(Live::Dab),
        };
        let begin = ms(t);
        let mut live = match live {
            Ok(l) => l,
            Err(e) => {
                println!("  {name}: begin failed: {e}");
                continue;
            }
        };
        let mut times = Vec::with_capacity(n);
        for q in &pts[1..] {
            let t = Instant::now();
            let r = match &mut live {
                Live::Brush(l) => l.push(&[sp(q)]),
                Live::Retouch(l) => l.push(&[sp(q)]),
                Live::Dab(l) => l.push(&[sp(q)]),
            };
            times.push(ms(t));
            if let Err(e) = r {
                println!("  {name}: push failed: {e}");
                break;
            }
        }
        let mut p = base.clone();
        p["points"] = json!(pts);
        if let Live::Brush(l) = live {
            p["seed"] = json!(l.seed);
            let _ = s.offer_live_stroke(*l);
        }
        let release = run(&mut s, cmd, p);
        let k = times.len();
        println!(
            "  {name:18} begin {begin:7.2} ms | per event first100 {:7.3} ms, last100 {:7.3} ms | release (commit) {release:8.1} ms",
            avg(&times[..100.min(k)]),
            avg(&times[k.saturating_sub(100)..])
        );
    }

    println!("== B. edits as history grows (6000x4000)");
    let mut s = doc(6000, 4000);
    let mut times = Vec::new();
    for i in 0..200 {
        let x = 100.0 + (i % 50) as f64 * 110.0;
        let y = 200.0 + (i / 50) as f64 * 800.0;
        times.push(run(&mut s, "paint.stroke", json!({"points":[[x,y],[x+60.0,y+40.0]],"size":30,"color":"#ff0000"})));
    }
    println!("  small stroke: first20 {:.2} ms, last20 {:.2} ms", avg(&times[..20]), avg(&times[180..]));
    let u: Vec<f64> = (0..20).map(|_| run(&mut s, "edit.undo", json!({}))).collect();
    let r: Vec<f64> = (0..20).map(|_| run(&mut s, "edit.redo", json!({}))).collect();
    println!("  undo avg {:.2} ms, redo avg {:.2} ms", avg(&u), avg(&r));

    println!("== C. layer count (3000x2000)");
    for layers in [10usize, 300] {
        let mut s = Session::new();
        s.execute("file.new", json!({"width":3000,"height":2000,"depth":8,"mode":"rgb"})).unwrap();
        for i in 0..layers {
            s.execute("layer.new.layer", json!({})).unwrap();
            let x = 50.0 + (i % 40) as f64 * 70.0;
            let y = 50.0 + (i / 40) as f64 * 200.0;
            let _ = s.execute("paint.stroke", json!({"points":[[x,y],[x+40.0,y+120.0]],"size":20,"color":"#336699"}));
        }
        let mut r = |c: &str, p: Value| -> f64 { avg(&(0..20).map(|_| run(&mut s, c, p.clone())).collect::<Vec<_>>()) };
        let stroke = r("paint.stroke", json!({"points":[[500,500],[540,560]],"size":20,"color":"#ff0000"}));
        let props = r("layer.setProps", json!({"opacity":50}));
        let newl = r("layer.new.layer", json!({}));
        let undo = r("edit.undo", json!({}));
        let redo = r("edit.redo", json!({}));
        let trans = r("layer.translate", json!({"dx":3,"dy":2}));
        let dup = r("layer.duplicate", json!({}));
        println!(
            "  {layers:4} layers: stroke {stroke:7.2} | setProps {props:7.2} | new layer {newl:7.2} | undo {undo:7.2} | redo {redo:7.2} | translate {trans:7.2} | duplicate {dup:7.2} ms"
        );
    }

    println!("== D. typing and lasso");
    let mut s = Session::new();
    s.execute("file.new", json!({"width":3000,"height":2000,"depth":8,"mode":"rgb"})).unwrap();
    let t = run(&mut s, "type.create", json!({"x":100,"y":200,"text":"a","size":24,"box":[100,100,2600,1800]}));
    println!("  type.create {t:.2} ms");
    let mut times = Vec::new();
    for i in 0..1500 {
        let ch = if i % 7 == 6 { " " } else { "x" };
        times.push(run(&mut s, "type.insertText", json!({"text": ch})));
    }
    println!("  insertText per key: first100 {:.2} ms, last100 {:.2} ms", avg(&times[..100]), avg(&times[1400..]));
    for count in [1_000usize, 10_000, 100_000] {
        let pts: Vec<[f64; 2]> = (0..count)
            .map(|i| {
                let a = i as f64 / count as f64 * std::f64::consts::TAU;
                [1500.0 + 900.0 * a.cos() * (1.0 + 0.05 * (a * 37.0).sin()), 1000.0 + 800.0 * a.sin()]
            })
            .collect();
        let t = run(&mut s, "select.lasso", json!({"points": pts}));
        println!("  select.lasso {count:6} points: {t:8.1} ms");
    }
}
