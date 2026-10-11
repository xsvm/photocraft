//! Fork-only analysis: where a keystroke's time goes as a paragraph grows.
use photocraft_doc::{LayerContent, LayerId};
use photocraft_engine::Session;
use serde_json::json;
use std::time::Instant;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn main() {
    for (label, point) in [("paragraph", false), ("point text", true)] {
        let mut s = Session::new();
        s.execute("file.new", json!({"width":3000,"height":2000,"depth":8,"mode":"rgb"})).unwrap();
        let create = if point {
            json!({"x":100,"y":200,"text":"a","size":24})
        } else {
            json!({"x":100,"y":200,"text":"a","size":24,"box":[100,100,2600,1800]})
        };
        let id = s.execute("type.create", create).unwrap()["layer"].as_u64().unwrap();
        let mut len = 1usize;
        for target in [200usize, 1000, 3000, 6000] {
            let chunk: String = (len..target).map(|i| if i % 7 == 6 { ' ' } else { 'x' }).collect();
            if !chunk.is_empty() {
                s.execute("type.edit", json!({"layer": id, "replace": {"start": len, "end": len, "text": chunk}})).unwrap();
                len = target;
            }
            // One keystroke the way the Type tool sends it (coalesced into one undo step).
            let reps = 10;
            let t = Instant::now();
            for k in 0..reps {
                s.execute("type.edit", json!({"layer": id, "replace": {"start": len, "end": len, "text": "y"}, "coalesce": 77})).unwrap();
                len += 1;
                let _ = k;
            }
            let key = ms(t) / reps as f64;
            let st = s.active().unwrap();
            let (dpi, fmt) = (st.doc.resolution_dpi, st.doc.pixel_format());
            let Some(LayerContent::Text(t)) = st.doc.layer(LayerId(id)).map(|l| &l.content) else { panic!("not text") };
            let t = t.clone();
            let mut eng = photocraft_text::shared().lock().unwrap();
            let a = Instant::now();
            let layout = eng.layout(&t, dpi);
            let lay = ms(a);
            let a = Instant::now();
            let (_l, r) = eng.render(&t, dpi, fmt);
            let render = ms(a);
            drop(eng);
            let a = Instant::now();
            let ty = photocraft_text::psd::build_tysh(&t, dpi, layout.bounds());
            let tysh = ms(a);
            println!(
                "{label:10} {len:5} chars: keystroke {key:7.2} ms | layout {lay:7.2} | layout+raster {render:7.2} | build_tysh {tysh:7.2} ms ({} B) | surface {:?}",
                ty.len(),
                r.surface.tile_bounds()
            );
        }
    }
}
