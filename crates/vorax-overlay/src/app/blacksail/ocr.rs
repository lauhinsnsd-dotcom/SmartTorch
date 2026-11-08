//! The seal screen's words with Windows' own text reader (Windows.Media.Ocr, as SmartTorch reads card titles):
//! a Chinese reader, Traditional first. The band where the cards' text is is read whole, its words with where
//! they are; which card each belongs to is worked out from them (`blacksail::cards`), so no resolution needs its
//! own places.

use ::blacksail::cards::{cards, Line, Word};
use ::blacksail::Img;
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;

pub struct Ocr {
    engine: OcrEngine,
    pub language: String,
}

const CHINESE: [&str; 8] = ["zh-Hant-TW", "zh-TW", "zh-Hant-HK", "zh-HK", "zh-Hant", "zh-Hans-CN", "zh-CN", "zh-Hans"];

/// The part of the screen read: the cards' titles to the 「確認」 under them (the first test's: 57-70% down,
/// 30-70% across, at 2560×1600), with room to spare for other screens.
const BAND: [f32; 4] = [0.08, 0.40, 0.92, 0.85];

/// The longest side Windows' reader takes.
const MAX_SIDE: f32 = 2600.0;

impl Ocr {
    pub fn new() -> windows::core::Result<Ocr> {
        for tag in CHINESE {
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

    /// The lines read in the screen's card band, their words placed in the screen's pixels. When its text is
    /// small (under 20 pixels high: 1920×1080, about 13), each card is read again on its own, enlarged to about 30
    /// pixels a character: Windows' reader can't take the whole band that big (2600 pixels at most), and at 1.2×
    /// it read 1920×1080's cards as nonsense (掉英墯的機率升).
    pub fn lines(&self, screen: &Img) -> windows::core::Result<Vec<Line>> {
        let (sw, sh) = (screen.width as f32, screen.height as f32);
        let band = [BAND[0] * sw, BAND[1] * sh, (BAND[2] - BAND[0]) * sw, (BAND[3] - BAND[1]) * sh];
        let first = self.read(screen, band, None)?;
        let mut heights: Vec<f32> = first.iter().flatten().map(|w| w.h).collect();
        heights.sort_by(f32::total_cmp);
        let Some(&text_h) = heights.get(heights.len() / 2) else { return Ok(first) };
        if text_h >= 20.0 {
            return Ok(first);
        }
        let cs = cards(&first, sw, sh);
        if cs.is_empty() {
            return Ok(first);
        }
        let mut xs: Vec<f32> = cs.iter().map(|c| c.x).collect();
        xs.sort_by(f32::total_cmp);
        let apart = xs.windows(2).map(|w| w[1] - w[0]).fold(f32::MAX, f32::min);
        let apart = if apart < f32::MAX { apart } else { 0.2 * sw };
        let mut out = Vec::new();
        for c in &cs {
            let th = (c.title[3] - c.title[1]).max(text_h);
            // its title to the 「確認」 under it: a card's width across, half a card's gap either side
            let rect = [c.x - apart / 2.0, c.title[1] - th / 2.0, apart, (c.title[3] - c.title[1]) + sh / 8.0 + 3.0 * th];
            out.extend(self.read(screen, rect, Some(30.0 / text_h))?);
        }
        Ok(if out.is_empty() { first } else { out })
    }

    /// The lines read in a part of the screen (x, y, width, height in its pixels), their words placed back in the
    /// screen's pixels. The part is made grey, dark text on light (the cards' text is light and coloured on dark),
    /// and enlarged `zoom` times, or to about 2000 pixels across when None; never past 2600 pixels.
    fn read(&self, screen: &Img, rect: [f32; 4], zoom: Option<f32>) -> windows::core::Result<Vec<Line>> {
        let x0 = rect[0].clamp(0.0, screen.width as f32) as usize;
        let y0 = rect[1].clamp(0.0, screen.height as f32) as usize;
        let w = (rect[2] as usize).min(screen.width - x0);
        let h = (rect[3] as usize).min(screen.height - y0);
        if w < 16 || h < 16 {
            return Ok(vec![]);
        }
        let scale = zoom.unwrap_or(2000.0 / w as f32).max(1.0).min(MAX_SIDE / w.max(h) as f32);
        let (ow, oh) = ((w as f32 * scale) as usize, (h as f32 * scale) as usize);
        let mut bgra = vec![255u8; ow * oh * 4];
        for y in 0..oh {
            let sy = y0 + ((y as f32 / scale) as usize).min(h - 1);
            for x in 0..ow {
                let sx = x0 + ((x as f32 / scale) as usize).min(w - 1);
                let i = (sy * screen.width + sx) * 4;
                let v = 255 - screen.data[i].max(screen.data[i + 1]).max(screen.data[i + 2]);
                let o = (y * ow + x) * 4;
                bgra[o..o + 3].copy_from_slice(&[v, v, v]);
            }
        }
        let bmp = SoftwareBitmap::CreateWithAlpha(BitmapPixelFormat::Bgra8, ow as i32, oh as i32, BitmapAlphaMode::Premultiplied)?;
        let writer = DataWriter::new()?;
        writer.WriteBytes(&bgra)?;
        bmp.CopyFromBuffer(&writer.DetachBuffer()?)?;
        let result = self.engine.RecognizeAsync(&bmp)?.get()?;
        let mut out = Vec::new();
        for line in result.Lines()? {
            let mut words = Vec::new();
            for word in line.Words()? {
                let r = word.BoundingRect()?;
                words.push(Word { text: word.Text()?.to_string(), x: x0 as f32 + r.X / scale, y: y0 as f32 + r.Y / scale, w: r.Width / scale, h: r.Height / scale });
            }
            out.push(words);
        }
        Ok(out)
    }
}
