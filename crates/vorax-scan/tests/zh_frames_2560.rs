//! Boards from a Chinese client 2560 wide (fixtures/zh-frames-2560): 16:10 (2560x1600) at the game's recommended
//! and smallest interface scales, the tanks, the round and the instruments read where the game puts them on a screen
//! narrower than 16:9; and 2560x1440, the tanks' icons told apart behind short names.

use std::path::PathBuf;
use vorax_scan::{scan, scan_data, Img};

fn decode(name: &str) -> Img {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/zh-frames-2560").join(name);
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

/// The tanks as "archetype rarity activity×quantity" ("-" empty), the round, the instruments and the scale read at.
fn read(name: &str) -> (Vec<String>, i32, Vec<String>, f64) {
    let r = scan(&decode(name), scan_data(), false);
    let tanks = r
        .state
        .tanks
        .iter()
        .map(|t| match t {
            Some(m) => format!("{} {} {}x{}", serde_json::to_value(m.archetype).unwrap().as_str().unwrap(), serde_json::to_value(m.rarity).unwrap().as_str().unwrap(), m.activity, m.quantity),
            None => "-".into(),
        })
        .collect();
    (tanks, r.state.round.unwrap_or(0), r.state.instruments.clone(), r.ui())
}

#[test]
fn recommended_scale_mid_game() {
    let (tanks, round, instruments, ui) = read("potion-r8-1600-recommended.png");
    assert_eq!(tanks, ["pest rare 77x192", "agito magic 5x57", "aemberon magic 5x108", "pest magic 5x75", "-", "-"]);
    assert_eq!(round, 8);
    assert_eq!(instruments, ["Chrysalis Specimen", "Nettle Toggle", "Tanned Leather Restraint"]);
    assert_eq!(ui, 1.25);
}

#[test]
fn recommended_scale_first_round_its_colours_left_alone() {
    // the screen just short of normal colours: corrected, its round wasn't sure at any scale
    let (tanks, round, instruments, ui) = read("potion-r1-1600-recommended.png");
    assert_eq!(tanks, ["pest magic 5x24", "boneSentry normal 1x36", "boneSentry magic 5x24", "-", "-", "-"]);
    assert_eq!(round, 1);
    assert_eq!(instruments, ["Chrysalis Specimen"]);
    assert_eq!(ui, 1.25);
}

#[test]
fn smallest_scale_six_tanks() {
    let (tanks, round, instruments, ui) = read("potion-r4-1600-smallest.png");
    assert_eq!(
        tanks,
        ["boneSentry magic 5x199", "pest magic 5x24", "aemberon normal 1x186", "aemberon normal 6x36", "boneSentry boss 305x1", "pest magic 10x24"]
    );
    assert_eq!(round, 4);
    assert_eq!(instruments, ["Contracted Claws"]);
    assert_eq!(ui, 1.0);
}

#[test]
fn smallest_scale_instrument_screen() {
    // read before as not the Surgical Preparation screen
    let (tanks, round, _, ui) = read("instrument-r4-1600-smallest.png");
    assert_eq!(tanks, ["boneSentry magic 5x199", "pest magic 5x24", "aemberon normal 1x186", "aemberon normal 6x36", "boneSentry boss 305x278", "-"]);
    assert_eq!(round, 4);
    assert_eq!(ui, 1.0);
}

#[test]
fn icons_over_short_names_at_1440() {
    // 弓兵 and 輕弩兵's gold helmets and 赤鏜螂's purple were read as Agito: the shelf behind the label, dimly lit,
    // joined the short names and moved the place looked at for the icon off it
    let (tanks, round, _, ui) = read("potion-r6-1440-recommended.png");
    assert_eq!(tanks, ["boneSentry magic 5x49", "agito magic 197x451", "boneSentry normal 1x36", "pest magic 5x59", "-", "-"]);
    assert_eq!(round, 6);
    assert_eq!(ui, 1.25);
}
