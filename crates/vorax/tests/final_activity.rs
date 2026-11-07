//! A protocol board's activity grown to the game's end (not in upstream).

use vorax::advisor::{final_activity, ScreenState};

const ROUND_12: &str = r#"{"tanks":[{"archetype":"pest","rarity":"normal","activity":252,"quantity":702},{"archetype":"boneSentry","rarity":"magic","activity":85,"quantity":94},{"archetype":"aemberon","rarity":"rare","activity":235,"quantity":678},{"archetype":"pest","rarity":"normal","activity":221,"quantity":576},{"archetype":"boneSentry","rarity":"normal","activity":206,"quantity":531},{"archetype":"agito","rarity":"normal","activity":206,"quantity":216}],"hand":["Live Leech Therapy"],"instruments":["Incubation Sac","Tanned Leather Restraint","Tanned Leather Restraint"],"instrumentRounds":[1,3,4],"phase":"protocol","round":12}"#;

/// Round 12's protocol board of a recorded game (Nov 8, 2026, 625,402) grown by one round is round 13's as read
/// (737,282); grown by both, the game ended past 840k, though its last board read showed 737,282.
#[test]
fn the_growth_after_the_last_board_is_counted() {
    let x: ScreenState = serde_json::from_str(ROUND_12).unwrap();
    let ua = |x: &ScreenState| x.tanks.iter().flatten().map(|m| m.activity as f64 * m.quantity as f64).sum::<f64>();
    assert_eq!(ua(&x), 625_402.0);
    let one_round = ScreenState { round: Some(13), ..x.clone() };
    assert_eq!(final_activity(&one_round, 64).round(), 737_282.0);
    assert!(final_activity(&x, 64) >= 840_000.0);
    // with no instrument nothing grows
    let bare = ScreenState { instruments: vec![], instrument_rounds: None, ..x };
    assert_eq!(final_activity(&bare, 64), 625_402.0);
}
