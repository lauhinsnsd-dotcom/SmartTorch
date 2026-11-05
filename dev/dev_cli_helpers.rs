// Dev helpers for crates/vorax-cli/src/main.rs (not shipped). Append the functions below to main.rs and add at the
// top of fn main():
//   if std::env::var("ZH").is_ok() { let a: Vec<String> = std::env::args().collect(); zh_read(&a[1]); return; }
//   if let Ok(lb) = std::env::var("ROUNDREF") { let a: Vec<String> = std::env::args().collect(); round_refs(&a[1], &lb); return; }
// ZH=1 vorax <frame.png>         : the Chinese title path with a stand-in OCR (dev/rocr.py, RapidOCR), prints phase + hand
// ROUNDREF=5 vorax <frame.png>   : the round digit's glyph vectors as JSON, for data/scan/digits_zh.json
// Run from the repo root. Build on Linux with: CARGO_PROFILE_RELEASE_LTO=false cargo build --release -p vorax-cli
pub fn zh_read(path: &str) {
    use vorax_scan::titles::{pictures_in, read_titles_in, Script};
    let frame = decode_png(path);
    let data = vorax_scan::scan_data();
    let img = match vorax_scan::window::windowed_game(&frame) { Some(a) => vorax_scan::window::crop_to(&frame, &a), None => frame };
    let names = vorax_scan::scan(&img, data, false);
    let (dy, ui) = (names.hand_dy.unwrap_or(0.0), names.ui());
    eprintln!("  img {}x{} dy={dy} ui={ui}", img.width, img.height);
    let pics = |phase: &str, size: usize, slots: &[usize]| pictures_in(&img, data, phase, size, slots, dy, ui, Script::Han);
    let mut n = 0;
    let mut read = |crop: &vorax_scan::Img| {
        let p = format!("/tmp/zh_title_{n}.png");
        n += 1;
        let f = std::fs::File::create(&p).unwrap();
        let mut e = png::Encoder::new(std::io::BufWriter::new(f), crop.width as u32, crop.height as u32);
        e.set_color(png::ColorType::Rgba);
        e.write_header().unwrap().write_image_data(&crop.data).unwrap();
        let t = String::from_utf8_lossy(&std::process::Command::new("python3").args(["dev/rocr.py", &p]).output().unwrap().stdout).to_string();
        eprintln!("  ocr: {:?}", t.lines().collect::<Vec<_>>());
        t
    };
    let r = read_titles_in(&img, names, data, &mut read, Some(&pics), Script::Han);
    println!("{:?} hand={:?}", r.state.phase, r.state.hand);
}

pub fn round_refs(path: &str, label: &str) {
    use vorax_scan::text::{glyphs, read_text, TextKind};
    use vorax_scan::pixels::{glyph_vector, gray, Rect};
    let frame = decode_png(path);
    let img = match vorax_scan::window::windowed_game(&frame) { Some(a) => vorax_scan::window::crop_to(&frame, &a), None => frame };
    let l = vorax_scan::layout::layout_at(img.width, img.height, 1.0);
    let g = gray(&img);
    let data = vorax_scan::scan_data();
    let mut best: Option<(f64, Rect)> = None;
    for dy in [0.0, 4.0, 8.0, 12.0, 16.0, 20.0, -4.0, -8.0] {
        for dx in [0.0, -6.0, 6.0, -12.0, 12.0] {
            let r = Rect::new(l.round.x + dx * l.k, l.round.y + dy * l.k, l.round.w, l.round.h);
            let gs = glyphs(&img, &r, TextKind::Round);
            if gs.len() != label.chars().count() { continue; }
            let s = read_text(&img, &g, &r, TextKind::Round, &data.digits["round"]).map_or(0.0, |t| t.score);
            let h: f64 = gs.iter().map(|b| b.h).sum();
            let q = h + s;
            if best.as_ref().is_none_or(|b| q > b.0) { best = Some((q, r)); }
        }
    }
    let Some((_, r)) = best else { eprintln!("{path}: no shift gives {label}"); return };
    let gs = glyphs(&img, &r, TextKind::Round);
    let line_h = gs.iter().fold(0.0f64, |m, b| m.max(b.h));
    let out: Vec<(String, Vec<f64>)> = label.chars().zip(&gs).map(|(c, b)| (c.to_string(), glyph_vector(&g, img.width, img.height, b, line_h).iter().map(|v| (*v as f64 * 1000.0).round() / 1000.0).collect())).collect();
    println!("{}", serde_json::to_string(&out).unwrap());
}
