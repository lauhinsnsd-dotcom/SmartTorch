//! A Chinese client's card titles, as a text reader might return them, matched to the cards (names_zh.json).

use vorax_scan::scan_data;
use vorax_scan::titles::{han_fit, han_pick};

/// The cards of one kind ranked for a read, best first, by their Chinese names.
fn ranked(phase: &str, text: &str) -> Vec<(String, f64)> {
    let data = scan_data();
    let names = match phase {
        "potion" => &data.names_zh.potion,
        "instrument" => &data.names_zh.instrument,
        _ => &data.names_zh.protocol,
    };
    let mut r: Vec<(String, f64)> = names.iter().map(|(en, zh)| (en.clone(), zh.iter().map(|z| han_fit(z, text, &data.names_zh.fold)).fold(f64::NEG_INFINITY, f64::max))).collect();
    r.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    r
}

/// Settled as the reader settles a title: at least 6 letters' worth, 3 clear of the next.
fn settled(phase: &str, text: &str) -> Option<String> {
    let r = ranked(phase, text);
    (r[0].1 >= 6.0 && r[0].1 - r[1].1 >= 3.0).then(|| r[0].0.clone())
}

#[test]
fn titles_read_in_traditional_simplified_or_mixed() {
    for (phase, text, card) in [
        ("instrument", "攣縮指爪", "Contracted Claws"),
        ("instrument", "挛缩指爪", "Contracted Claws"),
        ("instrument", "增生前额葉", "Hyperplastic Frontal Lobe"),
        ("instrument", "麟污刮骨刀", "Fouled Scraper"),
        ("instrument", "異形螫針", "Aberrant Stinger"),
        ("potion", "細肢藥粉-骨衛兵", "Slender-Limb Powder - Bone Sentry"),
        ("potion", "细肢藥粉-蠱蟲", "Slender-Limb Powder - Pest"),
        ("potion", "脊髓灰質溶液", "Gray Matter Spinal Solution"),
        ("potion", "生骨粉", "Osteogenic Powder"),
        ("potion", "稀有藥劑箱（小）", "Rare Potion Kit (Small)"),
        ("potion", "藥劑箱（小）", "Potion Kit (Small)"),
        ("potion", "異種萬靈藥：壯骨", "Xenopanacea: Osteotrophy"),
        ("potion", "異種萬靈藥：育雛", "Xenopanacea: Brood"),
        ("potion", "驚醒嗅鹽", "Rousing Salts"),
        ("potion", "誘蟲劑", "Insect Attractant"),
        ("potion", "第一儀式聖水：告解", "First Rite Holy Water: Confession"),
        ("potion", "第四儀式聖水：聖餐", "Ritual Holy Water: Flagellation"),
        ("protocol", "顱骨鑽孔術", "Trepanation"),
        ("protocol", "全身针灸療法", "Holistic Acupuncture Therapy"),
        ("protocol", "雷射截肢手术", "Laser Amputation"),
        ("protocol", "肌肉崧弛注射實", "Muscle Relaxant Injection Experiment"),
    ] {
        assert_eq!(settled(phase, text).as_deref(), Some(card), "{text}");
    }
}

#[test]
fn a_family_not_read_settles_neither_version() {
    // "细肢藥粉-" with the family hidden or not read: Pest or Bone Sentry, not to be guessed
    assert_eq!(settled("potion", "细肢藥粉-"), None);
    assert_eq!(settled("potion", "孿生激素"), None);
}

#[test]
fn body_text_alone_settles_nothing() {
    assert_eq!(settled("instrument", "回合結束時，根據"), None);
    assert_eq!(settled("potion", "選擇1組怪物"), None);
}

#[test]
fn a_title_with_characters_dropped_still_names_its_card() {
    let data = scan_data();
    for (phase, text, card) in [
        ("instrument", "人 示 本", Some("Chrysalis Specimen")),
        ("instrument", "人 本", Some("Chrysalis Specimen")),
        ("instrument", "人 一 本 / 合 結 束 時 , 根 據 當 前", Some("Chrysalis Specimen")),
        ("potion", "異 激 素", Some("Xeno-Hormone")),
        ("potion", "異 糧 激 素", Some("Xeno-Hormone")),
        ("potion", "麻 藥 / - 異 魔", Some("Anesthetic Tincture - Aemberon")),
        // not to be guessed: the family not read, too little left, or body text
        ("potion", "细肢藥粉-", None),
        ("potion", "醒 者", None),
        ("protocol", "表 驗", None),
        ("potion", "選擇1組怪物", None),
    ] {
        assert_eq!(han_pick(data, phase, text).as_deref(), card, "{text}");
    }
}

#[test]
fn a_card_not_in_the_data_is_known_as_such() {
    use vorax_scan::titles::han_unsupported;
    let data = scan_data();
    for text in ["解剖浸液：病軀", "解剖浸液:病躯", "解剖浸液病", "剖浸液：病軀"] {
        assert_eq!(han_unsupported(data, text).as_deref(), Some("解剖浸液：病軀"), "{text}");
    }
    // other titles, and ones too short to tell, are not it
    for text in ["", "浸液", "混合活蛭溶液", "脊髓灰質溶液", "細肢藥粉-", "黏稠膽汁溶液", "液"] {
        assert_eq!(han_unsupported(data, text), None, "{text}");
    }
    // and it matches no card of the data well enough to be settled as one
    assert_eq!(settled("potion", "解剖浸液：病軀"), None);
}

#[test]
fn a_title_a_fifth_ahead_is_picked() {
    // 軟腦膜溶液 3/5 against 消化酶溶液 2/5: a fifth ahead exactly (read live at 2732x1426, round 10)
    assert_eq!(han_pick(scan_data(), "potion", "腦 溶 液").as_deref(), Some("Leptomeningeal Solution"));
}

#[test]
fn a_longer_name_holding_more_of_the_read_is_picked() {
    // the last character cut off (read live at 2560x1440): Brood, not the plain Xenopanacea it holds
    assert_eq!(han_pick(scan_data(), "potion", "異 種 萬 靈 藥 : 育").as_deref(), Some("Xenopanacea: Brood"));
    assert_eq!(han_pick(scan_data(), "potion", "異種萬靈藥：壯").as_deref(), Some("Xenopanacea: Osteotrophy"));
    // the families' two versions still stay apart
    assert_eq!(han_pick(scan_data(), "potion", "孿生激素"), None);
    assert_eq!(han_pick(scan_data(), "potion", "細肢藥粉-"), None);
}

#[test]
fn the_simplified_clients_dash_read_as_a_character_is_left_out() {
    // the Simplified client's "孪生激素-骨卫兵", its dash read as 一 by a Traditional Chinese reader
    let data = scan_data();
    assert_eq!(settled("potion", "孪 生 激 素 一 骨 卫").as_deref(), Some("Twin Hormone - Bone Sentry"));
    assert_eq!(settled("potion", "脊 髓 溶 液 一 觉 醒 者").as_deref(), Some("Spinal Solution - Agito"));
    assert_eq!(han_pick(data, "potion", "麻 药 / 一 异 魔").as_deref(), Some("Anesthetic Tincture - Aemberon"));
    // still not to be guessed: the family not read
    assert_eq!(han_pick(data, "potion", "孪 生 激 素 一").as_deref(), None);
}

#[test]
fn the_chinese_clients_card_pictures_are_only_for_cards_upstream_has_none_of() {
    let data = scan_data();
    for key in ["instrument:Egg-Burdened Butterfly", "instrument:Aberrant Stinger"] {
        assert!(data.cards_zh.contains_key(key), "{key}");
    }
    assert!(data.cards_zh.keys().all(|k| !data.cards.contains_key(k)));
}

#[test]
fn characters_only_one_card_has_name_it_short_of_half() {
    let data = scan_data();
    // 疫區聖母像 with its statue over the middle of the title; 二寸顱骨釘 and 增生前額葉 half read
    assert_eq!(han_pick(data, "instrument", "疫 。 。 / 像").as_deref(), Some("Lady of the Plague Quarter"));
    assert_eq!(han_pick(data, "instrument", "寸 顱").as_deref(), Some("Two-Inch Skull Nail"));
    // characters another card of any kind has too name nothing: the potions' 膽汁 isn't 黑膽汁療法, 顱骨 is two cards'
    assert_eq!(han_pick(data, "protocol", "青 膽 汁 溶 液").as_deref(), None);
    assert_eq!(han_pick(data, "protocol", "顱 骨").as_deref(), None);
}

/// The cards a Xenopanacea: Brood leaves for later rounds (子一代 to 子三代, Supreme, not in upstream's data: always
/// taken), as a 2560x1440 client's titles were read (a player's log, 2026-11-07).
#[test]
fn the_brood_cards_are_told_apart() {
    let data = scan_data();
    for (text, card) in [
        ("子 一 代 • 爬 蟲 卵 簇", "Xenopanacea Brood: F1 Reptile Egg Cluster"),
        ("子 二 代 . 生 蟲 蛹", "Xenopanacea Brood: F2 Parasite Pupae"),
        ("子 二 代 : 爬 蟲 蛹 簇", "Xenopanacea Brood: F2 Reptile Pupa Cluster"),
        ("三 蟲 蛹 簇", "Xenopanacea Brood: F2 Reptile Pupa Cluster"),
        ("子 三 代 寄 生 蝶", "Xenopanacea Brood: F3 Parasitic Butterfly"),
        ("丨 子 三 弋 手 生 蝶", "Xenopanacea Brood: F3 Parasitic Butterfly"),
        ("子 三 代 • 破 繭", "Xenopanacea Brood: F3 Cocoon Swarm"),
        ("三 代 破 繭 蝶", "Xenopanacea Brood: F3 Cocoon Swarm"),
    ] {
        assert_eq!(han_pick(data, "potion", text).as_deref(), Some(card), "{text}");
        assert!(vorax::advisor::is_always_take(card), "{card}");
    }
    // too little to tell the two of a generation apart: not guessed
    assert_eq!(han_pick(data, "potion", "子 一 代 :"), None);
}
