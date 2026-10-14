
#[cfg(test)]
mod fork_fingerprint {
    use super::*;
    fn disc(w: usize, h: usize, r: f32, patchy: bool) -> Map {
        let mut m = Map::new(w, h, 0.0);
        for y in 0..h {
            for x in 0..w {
                let mut hit = 0;
                for sy in 0..8 {
                    for sx in 0..8 {
                        let (px, py) = (x as f32 + (sx as f32 + 0.5) / 8.0 - w as f32 / 2.0, y as f32 + (sy as f32 + 0.5) / 8.0 - h as f32 / 2.0);
                        hit += usize::from(px * px + py * py <= r * r);
                    }
                }
                let mut v = hit as f32 / 64.0;
                if patchy && (x / 13 + y / 17) % 5 == 0 { v *= 0.6; }
                m.v[y * w + x] = v;
            }
        }
        m
    }
    #[test]
    fn fork_bevel_fingerprint() {
        let mut sf = photocraft_raster::Surface::new(photocraft_color::PixelFormat::RGBA8);
        sf.fill_rect(Rect::new(0, 0, 4, 2), &[0.0, 0.0, 0.0, 1.0]);
        sf.fill_rect(Rect::new(0, 2, 4, 4), &[1.0, 1.0, 1.0, 1.0]);
        let pats = [Pattern::new("stripes", sf, 4, 4)];
        let light = GlobalLight::default();
        let mut all: u64 = 0xcbf29ce484222325;
        let mut n = 0;
        for (w, h, r, patchy) in [(97, 83, 30.0, false), (301, 257, 110.0, true), (420, 400, 150.0, false)] {
            let shape = disc(w, h, r, patchy);
            let tex = TextureCtx { rect: Rect::new(3, 5, 3 + w as i32, 5 + h as i32), patterns: &pats, anchor: (0.0, 0.0) };
            for style in [BevelStyle::InnerBevel, BevelStyle::OuterBevel, BevelStyle::Emboss, BevelStyle::PillowEmboss, BevelStyle::StrokeEmboss] {
                for technique in [BevelTechnique::Smooth, BevelTechnique::ChiselHard, BevelTechnique::ChiselSoft] {
                    for (size, soften, textured) in [(5.0f32, 0.0f32, false), (12.0, 2.0, false), (8.0, 0.0, true), (40.0, 0.0, false)] {
                        let mut b = Bevel { style, technique, size, soften, depth: 1.0, angle: 120.0, altitude: 30.0, use_global_light: false, ..Default::default() };
                        if textured {
                            b.texture = Some(photocraft_doc::BevelTexture { name: "stripes".into(), id: String::new(), scale: 1.0, depth: 1.0, invert: false, link: true, phase: (0.0, 0.0) });
                        }
                        let (maps, _) = bevel_maps(&shape, &b, &light, &tex, &PreparedPatterns::new(&pats, PREPARED_PATTERN_BYTES));
                        for m in maps {
                            for v in m.v { all = (all ^ u64::from(v.to_bits())).wrapping_mul(0x100000001b3); n += 1; }
                        }
                    }
                }
            }
        }
        println!("FORK_FP {all:016x} over {n} values");
    }
}
