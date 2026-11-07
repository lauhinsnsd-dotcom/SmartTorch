//! The player's home folder left out of what may be posted in public (the debug log, a report). Not in upstream
//! (DinoRaos/vorax-solver-rs v0.3.0 leaves it out of its debug log too).

/// `text` with the home folder `home` (as `C:\Users\name`, as JSON writes it, or with `/`) shown as
/// `%USERPROFILE%`, its case not minded (Windows doesn't).
pub fn hide_home(text: &str, home: &str) -> String {
    let home = home.trim_end_matches(['\\', '/']);
    if home.len() < 4 {
        return text.to_string();
    }
    let forms = [home.to_string(), home.replace('\\', "\\\\"), home.replace('\\', "/")];
    let mut out = text.to_string();
    for form in forms.iter().filter(|f| !f.is_empty()) {
        out = replace_ascii_nocase(&out, form, "%USERPROFILE%");
    }
    out
}

fn replace_ascii_nocase(text: &str, what: &str, with: &str) -> String {
    // ASCII lowercasing keeps every byte where it was, so the places found are the text's own
    let low = text.to_ascii_lowercase();
    let what = what.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while let Some(i) = low[at..].find(&what) {
        let end = at + i + what.len();
        // the whole folder's name, not the start of another's (C:\Users\chuck in C:\Users\chuckles)
        let whole = low[end..].chars().next().is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '.'));
        out.push_str(&text[at..at + i]);
        out.push_str(if whole { with } else { &text[at + i..end] });
        at = end;
    }
    out.push_str(&text[at..]);
    out
}

/// `hide_home` with this player's home folder (%USERPROFILE%).
pub fn private(text: &str) -> String {
    match std::env::var("USERPROFILE") {
        Ok(home) => hide_home(text, &home),
        Err(_) => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::hide_home;

    #[test]
    fn the_home_folder_is_left_out() {
        let home = r"C:\Users\chuck";
        assert_eq!(hide_home(r"report packed: C:\Users\chuck\OneDrive\Desktop\SmartTorch-report.zip", home), r"report packed: %USERPROFILE%\OneDrive\Desktop\SmartTorch-report.zip");
        assert_eq!(hide_home(r"c:\users\CHUCK\Downloads\x.exe", home), r"%USERPROFILE%\Downloads\x.exe");
        assert_eq!(hide_home("{\"path\":\"C:\\\\Users\\\\chuck\\\\Desktop\\\\\u{706b}\u{70ac}\u{5de5}\u{5177}\\\\a.exe\"}", home), "{\"path\":\"%USERPROFILE%\\\\Desktop\\\\\u{706b}\u{70ac}\u{5de5}\u{5177}\\\\a.exe\"}");
        assert_eq!(hide_home("C:/Users/chuck/AppData", home), "%USERPROFILE%/AppData");
        // a Chinese name (escaped: the font test reads every Chinese string here), other folders as they are
        assert_eq!(hide_home("C:\\Users\\\u{5c0f}\u{660e}\\Desktop \u{8207} E:\\Torchlight", "C:\\Users\\\u{5c0f}\u{660e}"), "%USERPROFILE%\\Desktop \u{8207} E:\\Torchlight");
        assert_eq!(hide_home("E:\\GAME\u{5de5}\u{5177}\\\u{706b}\u{70ac}\\\u{6613}\u{706b}.exe", home), "E:\\GAME\u{5de5}\u{5177}\\\u{706b}\u{70ac}\\\u{6613}\u{706b}.exe");
        assert_eq!(hide_home("anything", ""), "anything");
        assert_eq!(hide_home(r"C:\Users\chuckles\a", home), r"C:\Users\chuckles\a");
    }
}
