// An English title read in pieces at 1920x1080 (reads from a user's log, Nov 2026) names its card.
use vorax_scan::titles::word_pick;

#[test]
fn pieces_name_the_card() {
    let d = vorax_scan::scan_data();
    let i = &d.names.instrument;
    let p = &d.names.potion;
    assert_eq!(word_pick(i, "Claws\nWhen removing a\nmonster that is not").as_deref(), Some("Contracted Claws"));
    assert_eq!(word_pick(i, "Hyperp\nFront\nLobe\nWhen you have at le").as_deref(), Some("Hyperplastic Frontal Lobe"));
    assert_eq!(word_pick(i, "Prisjualic Liver\nWhen you have at le\n2 group(s) of").as_deref(), Some("Prismatic Liver"));
    assert_eq!(word_pick(p, "Potent\nPowder\nSelect 1 group(s) of").as_deref(), Some("Potent Xenopurge Powder"));
    // pieces two cards share name neither
    assert_eq!(word_pick(p, "Spinal Solution\nSelect 1 group(s) of"), None);
    assert_eq!(word_pick(p, "Select 1 group(s) of\nmonsters and +154 to"), None);
}
