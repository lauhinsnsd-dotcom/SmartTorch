//! The cards the player picked for titles the reader didn't make out (the picker; not in upstream): what the text
//! reader made of the title, and the card it was, by the kind of pick. Seen again, the same text names that card
//! by itself. Kept on this computer only (%APPDATA%\VoraxRs\learned.json), sent with a report.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Default, Clone)]
pub struct Learned {
    path: Option<PathBuf>,
    /// kind of pick ("potion", "instrument") → the title's text as read → the card's (English) name
    map: BTreeMap<String, BTreeMap<String, String>>,
}

/// A title's text as a key: the reader's spaces and line breaks don't count.
fn key(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace() && *c != '/').collect()
}

/// A card's name as the data has it now: picks learned before v2.0.0 may carry upstream's old names.
fn renamed(name: &str) -> &str {
    match name {
        "Fast-Acting Heart Tonic" => "Fast-Acting Cardiac Stimulant",
        "Purest Holy Water" => "The Holiest Water",
        n => n,
    }
}

impl Learned {
    pub fn load(path: &Path) -> Learned {
        let map = std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Learned { path: Some(path.to_path_buf()), map }
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        if self.map.is_empty() {
            let _ = std::fs::remove_file(path);
            return;
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.map) {
            let tmp = path.with_extension("json.new");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, path);
            }
        }
    }

    pub fn get(&self, phase: &str, text: &str) -> Option<String> {
        let k = key(text);
        if k.is_empty() {
            return None;
        }
        self.map.get(phase)?.get(&k).map(|n| renamed(n).to_string())
    }

    /// Remembers a pick (a title with no text is nothing to know it by again).
    pub fn put(&mut self, phase: &str, text: &str, name: &str) {
        let k = key(text);
        if k.is_empty() {
            return;
        }
        self.map.entry(phase.to_string()).or_default().insert(k, name.to_string());
        self.save();
    }

    pub fn count(&self) -> usize {
        self.map.values().map(|m| m.len()).sum()
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pick_names_the_same_text_again() {
        let dir = std::env::temp_dir().join(format!("vorax-learned-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("learned.json");
        let mut l = Learned::load(&path);
        assert_eq!(l.get("potion", "麻 、 異 魔"), None);
        l.put("potion", "麻 、 異 魔", "Anesthetic Tincture - Aemberon");
        l.put("potion", "", "Xeno-Hormone");
        assert_eq!(l.count(), 1);
        let again = Learned::load(&path);
        assert_eq!(again.get("potion", "麻、異魔").as_deref(), Some("Anesthetic Tincture - Aemberon"));
        assert_eq!(again.get("instrument", "麻、異魔"), None);
        // a pick learned under upstream's old name comes back under the new one
        let mut again = again;
        again.put("potion", "純聖", "Purest Holy Water");
        assert_eq!(again.get("potion", "純聖").as_deref(), Some("The Holiest Water"));
        again.clear();
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
