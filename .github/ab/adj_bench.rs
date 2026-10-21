//! Adjustment layers on the CPU compositor: a 4000×3000 photo-like 8-bit image with one
//! adjustment layer of each kind on top, `photocraft_compose::render` of the whole document,
//! min of 5. `adj_bench <kind>` runs one kind only (for a profile).
use photocraft_engine::Session;
use serde_json::{Value, json};
use std::time::Instant;

fn photo(w: u32, h: u32) -> Session {
    let mut px = vec![0u8; w as usize * h as usize * 3];
    let (cx, cy, r) = (w as f32 * 0.55, h as f32 * 0.5, w.min(h) as f32 / 3.0);
    for (y, row) in px.chunks_mut(w as usize * 3).enumerate() {
        for x in 0..w as usize {
            let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
            let n = ((x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663)) % 997) as f32 / 997.0 - 0.5;
            let blob = (-(((fx - 0.4) * 3.0).powi(2) + ((fy - 0.55) * 4.0).powi(2))).exp();
            let inside = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() < r;
            let (r0, g0, b0) = if inside { (0.85, 0.3 + 0.2 * (fx * 40.0).sin(), 0.15) } else { (0.35, 0.45, 0.55) };
            let rgb = [r0 + 0.1 * blob + n * 0.08, g0 + 0.1 * blob + n * 0.08, b0 + 0.2 * fy + n * 0.08];
            for (c, v) in rgb.into_iter().enumerate() {
                row[x * 3 + c] = (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
        }
    }
    let img = photocraft_codecs::Image::from_raw(w, h, photocraft_codecs::ChannelLayout::Rgb, photocraft_codecs::SampleType::U8, px).unwrap();
    let jpeg = photocraft_codecs::encode(&img, photocraft_codecs::Format::Jpeg, &Default::default()).unwrap();
    let doc = photocraft_io::import("photo.jpg", &jpeg).unwrap().document;
    let mut s = Session::new();
    s.open_document(doc, Some("photo.jpg".into()));
    s
}

fn sample(kind: &str) -> Value {
    match kind {
        "brightnessContrast" => json!({"brightness": 20, "contrast": -10}),
        "brightnessContrastLegacy" => json!({"brightness": 20, "contrast": -10, "legacy": true}),
        "threshold" => json!({"level": 100}),
        "posterize" => json!({"levels": 7}),
        "exposure" => json!({"exposure": 1.5, "offset": 0.1, "gamma": 1.4}),
        "vibrance" => json!({"vibrance": 30, "saturation": -20}),
        "hueSaturation" => json!({"hue": 25, "saturation": -20, "lightness": 10, "reds": {"hue": 10, "saturation": -30, "lightness": 5, "range": [300, 340, 20, 50]}}),
        "levels" => json!({"inBlack": 10, "inWhite": 240, "outBlack": 5, "outWhite": 250, "gamma": 1.2, "red": {"inBlack": 3, "gamma": 0.8}}),
        "curves" => json!({"points": [[0, 0], [128, 150], [255, 255]], "blue": [[0, 20], [255, 235]]}),
        "colorBalance" => json!({"shadows": [10, -5, 0], "midtones": [0, 20, -30], "highlights": [5, 5, 5], "preserveLuminosity": true}),
        "blackWhite" => json!({"reds": 120, "blues": -40, "tint": true, "tintColor": "#d0a070"}),
        "photoFilter" => json!({"color": "#3366cc", "density": 40, "preserveLuminosity": true}),
        "channelMixer" => json!({"red": [80, 30, -10, 5], "blue": [0, 10, 90, 0]}),
        "gradientMap" => json!({"stops": [[0, "#000000"], [0.5, "#ff0000"], [1, [1, 1, 1]]], "reverse": true, "dither": true}),
        "selectiveColor" => json!({"method": "absolute", "reds": [10, 0, -20, 5]}),
        _ => json!({}),
    }
}

fn main() {
    let only = std::env::args().nth(1);
    let kinds = [
        "none", "invert", "brightnessContrast", "brightnessContrastLegacy", "threshold", "posterize", "exposure", "vibrance", "hueSaturation",
        "levels", "curves", "colorBalance", "blackWhite", "photoFilter", "channelMixer", "gradientMap", "selectiveColor", "levelsMultiply60",
    ];
    let base = photo(4000, 3000);
    for kind in kinds.into_iter().filter(|k| only.as_deref().is_none_or(|o| o == *k)) {
        let mut s = Session::new();
        s.open_document((*base.active().unwrap().doc).clone(), None);
        if kind != "none" {
            let id = match kind {
                "brightnessContrastLegacy" => "brightnessContrast",
                "levelsMultiply60" => "levels",
                k => k,
            };
            s.execute(&format!("layer.newAdjustmentLayer.{id}"), sample(if kind == "levelsMultiply60" { "levels" } else { kind })).unwrap();
        }
        let mut doc = (*s.active().unwrap().doc).clone();
        if kind == "levelsMultiply60" {
            let top = doc.layers.last_mut().unwrap();
            top.blend = photocraft_color::blend::BlendMode::Multiply;
            top.opacity = 0.6;
        }
        let mut best = f64::MAX;
        let mut print = 0u64;
        for _ in 0..5 {
            let t = Instant::now();
            let out = std::hint::black_box(photocraft_compose::render(&doc, doc.bounds()));
            best = best.min(t.elapsed().as_secs_f64() * 1000.0);
            print = out.px.iter().flatten().fold(0xcbf2_9ce4_8422_2325, |h, v| (h ^ u64::from(v.to_bits())).wrapping_mul(0x100_0000_01b3));
        }
        println!("{kind:26} min {best:8.1} ms  fingerprint {print:016x}");
    }
}
