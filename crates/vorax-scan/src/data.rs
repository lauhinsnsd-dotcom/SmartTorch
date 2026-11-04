//! The reader's reference data (data/scan: data.json, learnedIcons.json, levels.json, and the names titles are
//! matched against), as tli-vorax-simulator last published it. Embedded at build time.

use crate::pixels::{Art, ArtFeature};
use base64::Engine;
use indexmap::IndexMap;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Packed {
    digits: IndexMap<String, IndexMap<String, Vec<Vec<f64>>>>,
    cards: IndexMap<String, Vec<ArtFeature>>,
    texts: IndexMap<String, Vec<String>>,
    headers: IndexMap<String, Vec<Vec<f64>>>,
    instrument_icons: IndexMap<String, Vec<Vec<f64>>>,
}

#[derive(Deserialize)]
pub struct Names {
    pub potion: Vec<String>,
    pub instrument: Vec<String>,
    pub protocol: Vec<String>,
}

/// The Chinese names a Chinese client prints for the cards (names_zh.json): per kind of pick, each card's English
/// name to the Simplified and Traditional forms it may be read as. Cards not in it are left to their pictures.
#[derive(Deserialize, Default)]
pub struct NamesZh {
    pub potion: IndexMap<String, Vec<String>>,
    pub instrument: IndexMap<String, Vec<String>>,
    pub protocol: IndexMap<String, Vec<String>>,
    /// Traditional (and variant) characters to their Simplified form, for matching a read that mixes the two
    #[serde(default)]
    pub fold: std::collections::HashMap<char, char>,
    /// cards a Chinese client shows that upstream's data doesn't have (the Solver can't weigh them), as printed
    #[serde(default)]
    pub unsupported: Vec<String>,
}

pub struct ScanData {
    /// glyph vectors by text kind ("label", "round", "refresh"), then character
    pub digits: IndexMap<String, IndexMap<String, Vec<Vec<f64>>>>,
    /// card art by "phase:name", centred for comparing
    pub cards: IndexMap<String, Vec<Art>>,
    /// card text-block shapes by "phase:name"
    pub texts: IndexMap<String, Vec<Vec<f64>>>,
    /// header text shape for each phase
    pub headers: IndexMap<String, Vec<Vec<f64>>>,
    /// equipped-instrument panel icons (built in, then the ones players taught)
    pub icons: IndexMap<String, Vec<Vec<f64>>>,
    /// a normal capture's levels (levels.json)
    pub levels: Vec<f64>,
    pub names: Names,
    /// the cards' Chinese names, for reading a Chinese client
    pub names_zh: NamesZh,
    /// more glyph references a Chinese client's text needs (digits_zh.json): by text kind, then character
    pub digits_zh: IndexMap<String, IndexMap<String, Vec<Vec<f64>>>>,
}

/// Card text shapes are packed one signed byte per value, in steps of this.
const TEXT_STEP: f64 = 0.002;

impl ScanData {
    pub fn parse(data: &str, learned: &str, levels: &str, names: &str) -> Result<ScanData, String> {
        let p: Packed = serde_json::from_str(data).map_err(|e| format!("scan data.json: {e}"))?;
        let learned: IndexMap<String, Vec<Vec<f64>>> = serde_json::from_str(learned).map_err(|e| format!("learnedIcons.json: {e}"))?;
        let b64 = base64::engine::general_purpose::STANDARD;
        let mut texts = IndexMap::new();
        for (k, vs) in p.texts {
            let mut out = Vec::new();
            for s in vs {
                let bytes = b64.decode(s.as_bytes()).map_err(|e| format!("scan text {k}: {e}"))?;
                out.push(bytes.iter().map(|&b| (b as i8) as f64 * TEXT_STEP).collect());
            }
            texts.insert(k, out);
        }
        let mut icons = p.instrument_icons;
        for (name, vs) in learned {
            icons.entry(name).or_default().extend(vs);
        }
        Ok(ScanData {
            digits: p.digits,
            cards: p.cards.into_iter().map(|(k, fs)| (k, fs.iter().map(Art::from).collect())).collect(),
            texts,
            headers: p.headers,
            icons,
            levels: serde_json::from_str(levels).map_err(|e| format!("levels.json: {e}"))?,
            names: serde_json::from_str(names).map_err(|e| format!("names.json: {e}"))?,
            names_zh: NamesZh::default(),
            digits_zh: IndexMap::new(),
        })
    }

    /// With more glyph references for a Chinese client (digits_zh.json).
    pub fn with_digits_zh(mut self, digits_zh: &str) -> Result<ScanData, String> {
        #[derive(Deserialize)]
        struct Packed {
            round: IndexMap<String, Vec<Vec<f64>>>,
            #[serde(default)]
            label: IndexMap<String, Vec<Vec<f64>>>,
        }
        let p: Packed = serde_json::from_str(digits_zh).map_err(|e| format!("digits_zh.json: {e}"))?;
        self.digits_zh.insert("round".into(), p.round);
        self.digits_zh.insert("label".into(), p.label);
        Ok(self)
    }

    /// A kind of text's references with the Chinese client's added.
    pub fn digits_with_zh(&self, kind: &str) -> IndexMap<String, Vec<Vec<f64>>> {
        let mut refs = self.digits.get(kind).cloned().unwrap_or_default();
        for (c, vs) in self.digits_zh.get(kind).into_iter().flatten() {
            refs.entry(c.clone()).or_default().extend(vs.iter().cloned());
        }
        refs
    }

    /// With the cards' Chinese names (names_zh.json).
    pub fn with_names_zh(mut self, names_zh: &str) -> Result<ScanData, String> {
        self.names_zh = serde_json::from_str(names_zh).map_err(|e| format!("names_zh.json: {e}"))?;
        Ok(self)
    }

    pub fn embedded() -> Result<ScanData, String> {
        ScanData::parse(
            include_str!("../../../data/scan/data.json"),
            include_str!("../../../data/scan/learnedIcons.json"),
            include_str!("../../../data/scan/levels.json"),
            include_str!("../../../data/scan/names.json"),
        )?
        .with_names_zh(include_str!("../../../data/scan/names_zh.json"))?
        .with_digits_zh(include_str!("../../../data/scan/digits_zh.json"))
    }

}

static DATA: OnceLock<ScanData> = OnceLock::new();

pub fn scan_data() -> &'static ScanData {
    DATA.get_or_init(|| ScanData::embedded().unwrap_or_else(|e| panic!("{e}")))
}
