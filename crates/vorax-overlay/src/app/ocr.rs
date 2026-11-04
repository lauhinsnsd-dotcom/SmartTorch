//! Card titles read with Windows' own text reader (Windows.Media.Ocr): English when that language pack is
//! installed, else the first language of the user's profile (Latin letters read fine either way; the names
//! are matched loosely). Upstream reads them with Tesseract; when neither is there, the pictures decide.
//! For a Chinese client, --ocr-lang zh (or a tag like zh-Hant-TW) reads them in Chinese; a Chinese reader is also
//! what's used when Windows has no English one and the profile's language is Chinese.

use vorax_scan::pixels::Img;
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use vorax_scan::titles::Script;

pub struct Ocr {
    engine: OcrEngine,
    pub language: String,
}

impl Ocr {
    pub fn new() -> windows::core::Result<Ocr> {
        Ocr::with_language(None)
    }

    /// `prefer`: a language tag to read in when Windows has its text recognition ("zh" for any Chinese one, the
    /// Traditional ones first); otherwise, or when it isn't installed, as `new`.
    pub fn with_language(prefer: Option<&str>) -> windows::core::Result<Ocr> {
        let tags: Vec<String> = match prefer {
            Some(p) if p.eq_ignore_ascii_case("zh") => ["zh-Hant-TW", "zh-TW", "zh-Hant-HK", "zh-HK", "zh-Hant", "zh-Hans-CN", "zh-CN", "zh-Hans"].iter().map(|t| t.to_string()).collect(),
            Some(p) => vec![p.to_string()],
            None => vec![],
        };
        for tag in tags.iter().map(String::as_str).chain(std::iter::once("en-US")) {
            let Ok(lang) = Language::CreateLanguage(&HSTRING::from(tag)) else { continue };
            if OcrEngine::IsLanguageSupported(&lang).unwrap_or(false) {
                if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&lang) {
                    let language = engine.RecognizerLanguage()?.LanguageTag()?.to_string();
                    return Ok(Ocr { engine, language });
                }
            }
        }
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
        let language = engine.RecognizerLanguage()?.LanguageTag()?.to_string();
        Ok(Ocr { engine, language })
    }

    /// How the card titles it reads are matched: Chinese ones to the cards' Chinese names.
    pub fn script(&self) -> Script {
        if self.language.to_ascii_lowercase().starts_with("zh") {
            Script::Han
        } else {
            Script::Latin
        }
    }

    /// The text in a picture (dark text on white), line by line.
    pub fn read(&self, img: &Img) -> windows::core::Result<String> {
        let bmp = SoftwareBitmap::CreateWithAlpha(BitmapPixelFormat::Bgra8, img.width as i32, img.height as i32, BitmapAlphaMode::Premultiplied)?;
        let mut bgra = img.data.clone();
        for p in bgra.chunks_exact_mut(4) {
            p.swap(0, 2);
        }
        let writer = DataWriter::new()?;
        writer.WriteBytes(&bgra)?;
        bmp.CopyFromBuffer(&writer.DetachBuffer()?)?;
        let result = self.engine.RecognizeAsync(&bmp)?.get()?;
        let mut lines = Vec::new();
        for line in result.Lines()? {
            lines.push(line.Text()?.to_string());
        }
        Ok(lines.join("\n"))
    }
}
