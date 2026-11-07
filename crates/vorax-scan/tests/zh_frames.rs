//! A Chinese client's boards as played (fixtures/zh-frames), their titles as Windows' reader returned them in the
//! game's log, read the way the overlay reads them: the interface scale found, the cards named.

use std::path::PathBuf;
use vorax_scan::titles::{pictures_in, read_titles_in, Script};
use vorax_scan::{scan, scan_data, Img};

fn decode(name: &str) -> Img {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/zh-frames").join(name);
    let mut dec = png::Decoder::new(std::fs::File::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let (w, h) = (info.width as usize, info.height as usize);
    let ch = info.color_type.samples();
    let mut img = Img::new(w, h);
    for i in 0..w * h {
        let p = &buf[i * ch..i * ch + ch];
        let px = if ch == 3 { [p[0], p[1], p[2], 255] } else { [p[0], p[1], p[2], p[3]] };
        img.data[i * 4..i * 4 + 4].copy_from_slice(&px);
    }
    img
}

/// The hand read with the titles' first reads given (one per card, left to right) and every read again returning
/// nothing, as Windows' reader did; on a fresh reader (100% interface scale, none read for sure yet).
fn hand(name: &str, titles: &[&str]) -> (f64, Vec<String>) {
    let data = scan_data();
    let img = decode(name);
    let names = scan(&img, data, false);
    let (dy, ui) = (names.hand_dy.unwrap_or(0.0), names.ui());
    let pics = |phase: &str, size: usize, slots: &[usize]| pictures_in(&img, data, phase, size, slots, dy, ui, Script::Han);
    let mut given = titles.iter();
    let mut read = |_: &Img| given.next().map_or(String::new(), |t| t.to_string());
    let r = read_titles_in(&img, names, data, &mut read, Some(&pics), Script::Han);
    (r.ui(), r.state.hand)
}

#[test]
fn a_potion_kit_whose_size_isnt_read_is_told_by_its_picture() {
    // 1920x1080 at the game's recommended interface scale (about 125%): 藥劑箱（大） read without its "（大）"
    let (ui, hand) = hand("potion-kit-large-1080p-ui125.png", &["藥 劑 箱 ~", "細 肢 : 蠱 蟲", "化 骨 油 膏"]);
    assert_eq!(ui, 1.25);
    assert_eq!(hand, ["Potion Kit (Large)", "Slender-Limb Powder - Pest", "Osteolytic Ointment"]);
}

#[test]
fn a_twin_hormone_told_by_its_versions_own_character() {
    // 孿生激素-骨衛兵 read as "孿 生 - 骨": 骨 is only the Bone Sentry version's
    let (ui, hand) = hand("twin-hormone-1080p-ui125.png", &["靶 向 異 種 激 素", "孿 生 - 骨", "活 殖 藥 粉"]);
    assert_eq!(ui, 1.25);
    assert_eq!(hand, ["Targeted Xeno-Hormone", "Twin Hormone - Bone Sentry", "Vivigenic Powder"]);
}
