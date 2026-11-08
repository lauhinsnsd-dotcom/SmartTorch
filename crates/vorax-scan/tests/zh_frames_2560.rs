//! Boards from a Chinese client 2560 wide (fixtures/zh-frames-2560): 16:10 (2560x1600) at the game's recommended
//! and smallest interface scales, the tanks, the round and the instruments read where the game puts them on a screen
//! narrower than 16:9; 2560x1440, the tanks' icons told apart behind short names; and one 3840x2160 board at the
//! smallest scale, its refreshes left.

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

/// The hand read with the titles' first reads given (one per card, left to right) and every read again returning
/// nothing, the way the overlay reads it.
fn hand(name: &str, titles: &[&str]) -> Vec<String> {
    use vorax_scan::titles::{pictures_in, read_titles_in, Script};
    let data = scan_data();
    let img = decode(name);
    let names = scan(&img, data, false);
    let (dy, ui) = (names.hand_dy.unwrap_or(0.0), names.ui());
    let pics = |phase: &str, size: usize, slots: &[usize]| pictures_in(&img, data, phase, size, slots, dy, ui, Script::Han);
    let mut given = titles.iter();
    let mut read = |_: &Img| given.next().map_or(String::new(), |t| t.to_string());
    read_titles_in(&img, names, data, &mut read, Some(&pics), Script::Han).state.hand
}

/// The middle card's picture in a hand of five, best match first.
fn middle_picture(name: &str) -> String {
    use vorax_scan::titles::{pictures_in, Script};
    let data = scan_data();
    let img = decode(name);
    let names = scan(&img, data, false);
    let p = pictures_in(&img, data, "potion", 5, &[2], names.hand_dy.unwrap_or(0.0), names.ui(), Script::Han);
    p[&2].ranks.first().map(|r| r.0.clone()).unwrap_or_default()
}

#[test]
fn a_hand_of_five_at_the_recommended_scale() {
    // the hand sits lower and further left than at 100%: the middle card's title was cut off at the bottom of the
    // place read, and never read (its neighbour's read short: 靶向異種激素 as 異種激素)
    assert_eq!(middle_picture("potion-hand5-1600-recommended.png"), "Twin Hormone - Pest");
    let h = hand("potion-hand5b-1600-recommended.png", &["脊 髓 灰 質 溶 液", "腦 霧 酊 劑", "細 肢 藥 粉 一 蠱 蟲", "強 效 祛 異 藥 粉", "細 肢 藥 粉 - 骨 衛 兵"]);
    assert_eq!(h, ["Gray Matter Spinal Solution", "Brain Fog Tincture", "Slender-Limb Powder - Pest", "Potent Xenopurge Powder", "Slender-Limb Powder - Bone Sentry"]);
    let h = hand("potion-hand5-1600-recommended.png", &["麻 藥 酊 劑 - 覺 醒 者", "靶 向 異 種 激 素", "孿 生 激 素 一 蠱 蟲", "卵 殼 藥 粉", "化 骨 油 膏"]);
    assert_eq!(h, ["Anesthetic Tincture - Agito", "Targeted Xeno-Hormone", "Twin Hormone - Pest", "Eggshell Powder", "Osteolytic Ointment"]);
}

#[test]
fn a_title_read_without_its_first_characters_is_told_by_the_picture() {
    // 靶向異種激素 read as 異種激素 (Windows' reader left out 靶向, the title whole in the place read), at 2560x1600 and
    // as it would be at 2560x1440: the picture is 靶向異種激素's
    let h = hand("targeted-xeno-1600-recommended.png", &["誘 變 藥 粉", "脊 髓 溶 液 一 覺 醒 者", "異 種 激 素"]);
    assert_eq!(h, ["Mutagenic Powder", "Spinal Solution - Agito", "Targeted Xeno-Hormone"]);
    let h = hand("targeted-xeno-1440-recommended.png", &["軟 腦 膜 溶 液", "純 粹 活 蛭 溶 液", "異 種 激 素"]);
    assert_eq!(h, ["Leptomeningeal Solution", "Pure Leech Solution", "Targeted Xeno-Hormone"]);
    // and 異種激素 itself stays itself
    // (one interface scale after the other, as a player's boards come: a reader keeps to the scale it last read at)
    let h = hand("xeno-1440-recommended.png", &["異 種 激 素", "脊 髓 溶 液 一 異 魔", "蛻 生 皮 溶 液"]);
    assert_eq!(h, ["Xeno-Hormone", "Spinal Solution - Aemberon", "Slough Solution"]);
    let h = hand("xeno-1600-smallest.png", &["蛻 生 皮 溶 液", "細 肢 藥 粉 一 骨 衛 兵", "異 種 激 素"]);
    assert_eq!(h, ["Slough Solution", "Slender-Limb Powder - Bone Sentry", "Xeno-Hormone"]);
}

#[test]
fn refreshes_left_at_3840_smallest_scale() {
    // 3840x2160 at the smallest scale: the count on the refresh button is drawn taller against it than upstream
    // measured, and was read as none left (0) on every board
    use vorax_scan::titles::{read_titles_in, Script};
    let data = scan_data();
    let img = decode("potion-r1-2160-smallest.png");
    let r = scan(&img, data, false);
    assert_eq!(r.state.potion_refreshes, Some(3));
    assert!(r.fields["refreshes"].sure);
    let titles = ["\u{810a}\u{9ad3}\u{6eb6}\u{6db2}\u{4e00}\u{89c9}\u{9192}", "\u{5b6a}\u{751f}\u{6fc0}\u{7d20}\u{4e00}\u{86ca}\u{866b}", "\u{77f3}\u{5316}\u{810a}\u{9ad3}\u{6eb6}"];
    let mut given = titles.iter();
    let mut read = |_: &Img| given.next().map_or(String::new(), |t| t.to_string());
    let read = read_titles_in(&img, r, data, &mut read, None, Script::Han);
    assert_eq!(read.state.hand, ["Spinal Solution - Agito", "Twin Hormone - Pest", "Petrified Spinal Solution"]);
    assert_eq!(read.state.potion_refreshes, Some(3));
    assert_eq!(read.state.round, Some(1));
}
