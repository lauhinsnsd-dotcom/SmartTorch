//! The seal screen's cards from the words read on it: found by their text, not by where they sit, so any
//! resolution reads the same way. The words fall in columns, one a card (a white card's dim title can go unread,
//! so the columns come from all the words, not the titles); in each, the title ("財富秘語") names the category and
//! the words under it are the seal's text.
//!
//! A screen is read many times while it's up (`Board`): each card keeps its best reading, so a frame read badly
//! (the cards still being dealt, a line missed) doesn't lose one read well.

use crate::seals::{category, identify, normalise, Seal};
use crate::Img;

/// A word read, with where it is (screen pixels).
#[derive(Debug, Clone)]
pub struct Word {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// A line read: its words, left to right.
pub type Line = Vec<Word>;

/// A run of words close together on one line (a line can run across cards).
#[derive(Debug, Clone)]
struct Piece {
    text: String,
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
}

impl Piece {
    fn cx(&self) -> f32 {
        (self.x0 + self.x1) / 2.0
    }
}

/// The lines cut where the gap between words is over their height: within a card's line the gap is a space at
/// most; between two cards' lines it's over twice the text's height (50 against 22 at 2560×1600).
fn pieces(lines: &[Line]) -> Vec<Piece> {
    let mut out = Vec::new();
    for line in lines {
        let mut words: Vec<&Word> = line.iter().filter(|w| !w.text.trim().is_empty()).collect();
        words.sort_by(|a, b| a.x.total_cmp(&b.x));
        let mut cur: Option<Piece> = None;
        for w in words {
            if let Some(p) = &mut cur {
                if w.x - p.x1 <= w.h.max(p.y1 - p.y0) {
                    p.text.push_str(&w.text);
                    p.x1 = p.x1.max(w.x + w.w);
                    p.y0 = p.y0.min(w.y);
                    p.y1 = p.y1.max(w.y + w.h);
                    continue;
                }
                out.push(cur.take().unwrap());
            }
            cur = Some(Piece { text: w.text.clone(), x0: w.x, x1: w.x + w.w, y0: w.y, y1: w.y + w.h });
        }
        out.extend(cur);
    }
    out
}

/// One card on the screen.
#[derive(Debug, Clone)]
pub struct Card {
    /// 財富, 軍械 or 奇珍; None when its title wasn't read
    pub category: Option<&'static str>,
    /// its text as read
    pub text: String,
    pub doubled: bool,
    /// "確認" under it: the card chosen
    pub chosen: bool,
    /// its column's middle
    pub x: f32,
    /// its title's box (left, top, right, bottom); where it would be when it wasn't read
    pub title: [f32; 4],
}

impl Card {
    /// The seal it is, given its rarity when known.
    pub fn seal(&self, rarity: Option<u8>) -> Option<&'static Seal> {
        identify(self.category, &self.text, rarity, self.doubled)
    }
}

fn is_title(p: &Piece) -> Option<&'static str> {
    (normalise(&p.text).chars().count() <= 6).then(|| category(&p.text)).flatten()
}

/// The pieces made into cards. A card's line can be read in parts (small text: a space around a number wider
/// than the text is high), a part off its card's middle. The pieces with a title
/// are cards; the distance between titles is a card's width; any other run within half of it of a card is part
/// of that card, else a card of its own (a white card's dim title unread), the biggest such runs first.
fn gather(runs: Vec<Vec<&Piece>>, width: f32) -> (Vec<Vec<&Piece>>, f32) {
    let mid = |r: &Vec<&Piece>| r.iter().map(|p| p.cx()).sum::<f32>() / r.len() as f32;
    let (mut cards, mut rest): (Vec<Vec<&Piece>>, Vec<Vec<&Piece>>) = runs.into_iter().partition(|r| r.iter().any(|p| is_title(p).is_some()));
    let mut xs: Vec<f32> = cards.iter().map(|r| r.iter().find(|p| is_title(p).is_some()).map_or(mid(r), |t| t.cx())).collect();
    let mut sorted = xs.clone();
    sorted.sort_by(f32::total_cmp);
    // two titles with text halfway between them (a card whose title went unread) are two cards apart; a part of
    // a titled card's line reaches at most a third of the way (0.36 in the fifth test)
    let big: Vec<f32> = rest.iter().filter(|r| r.iter().map(|p| p.text.chars().count()).sum::<usize>() >= 5).map(mid).collect();
    let gaps: Vec<f32> = sorted.windows(2).map(|w| w[1] - w[0]).collect();
    let least = gaps.iter().copied().fold(f32::MAX, f32::min);
    let apart = sorted
        .windows(2)
        .zip(&gaps)
        .map(|(w, &gap)| {
            // with other gaps to go by, only one clearly wider than the least can be two (a part of a neighbour's
            // line can sit halfway between titles one gap apart: the ninth test, 1920×1080)
            let can_be_two = gaps.len() == 1 || gap >= 1.6 * least;
            if can_be_two && big.iter().any(|m| *m > w[0] + 0.4 * gap && *m < w[1] - 0.4 * gap) {
                gap / 2.0
            } else {
                gap
            }
        })
        .fold(f32::MAX, f32::min);
    // one title or none: cards are 15-23% of the screen's width apart in the tests (smallest to recommended
    // interface scale)
    let apart = if apart < f32::MAX { apart } else { 0.15 * width };
    rest.sort_by_key(|r| std::cmp::Reverse(r.iter().map(|p| p.text.chars().count()).sum::<usize>()));
    for r in rest {
        let m = mid(&r);
        match (0..xs.len()).min_by(|a, b| (xs[*a] - m).abs().total_cmp(&(xs[*b] - m).abs())).filter(|i| (xs[*i] - m).abs() < apart / 2.0) {
            Some(i) => cards[i].extend(r),
            None => {
                xs.push(m);
                cards.push(r);
            }
        }
    }
    let mut order: Vec<usize> = (0..cards.len()).collect();
    order.sort_by(|a, b| xs[*a].total_cmp(&xs[*b]));
    let mut taken: Vec<Option<Vec<&Piece>>> = cards.into_iter().map(Some).collect();
    (order.into_iter().filter_map(|i| taken[i].take()).collect(), apart)
}

/// The cards on a screen `width` × `height` from its lines read, left to right.
pub fn cards(lines: &[Line], width: f32, height: f32) -> Vec<Card> {
    let ps: Vec<Piece> = pieces(lines)
        .into_iter()
        .filter(|p| {
            let t = normalise(&p.text);
            !t.contains("請選擇") && !(t.contains("封印") && t.chars().count() > 5)
        })
        .collect();
    // the titles' row: every card's title is on it
    let title_row: Option<(f32, f32)> = {
        let ts: Vec<&Piece> = ps.iter().filter(|p| is_title(p).is_some()).collect();
        (!ts.is_empty()).then(|| (ts.iter().map(|p| p.y0).sum::<f32>() / ts.len() as f32, ts.iter().map(|p| p.y1).sum::<f32>() / ts.len() as f32))
    };
    // columns: pieces whose middles are within 7% of the screen's width of one another (cards are 18% apart)
    // each piece on its own: grouping pieces by their middles first joined the end of one card's line to the
    // start of the next card's (the sixth test, small text read in parts)
    let runs: Vec<Vec<&Piece>> = ps.iter().map(|p| vec![p]).collect();
    let (columns, apart) = gather(runs, width);
    let mut out = Vec::new();
    for col in columns {
        let title = col.iter().copied().find(|p| is_title(p).is_some());
        let (ty0, ty1) = title.map(|t| (t.y0, t.y1)).or(title_row).unwrap_or_else(|| {
            // no title anywhere: the text starts just under where it would be
            let top = col.iter().map(|p| p.y0).fold(f32::MAX, f32::min);
            (top - 0.04 * height, top - 0.02 * height)
        });
        let mut card = Card { category: title.and_then(is_title), text: String::new(), doubled: false, chosen: false, x: 0.0, title: [0.0, ty0, 0.0, ty1] };
        let mut body: Vec<&Piece> = Vec::new();
        for p in &col {
            let t = normalise(&p.text);
            if t.contains("確認") {
                card.chosen = true;
            } else if title.is_some_and(|tt| std::ptr::eq(*p, tt)) || p.y0 < ty1 - 1.0 || p.y0 > ty1 + height / 8.0 {
                continue;
            } else if p.text.contains("翻倍") || (p.text.contains('效') && p.text.contains('倍')) {
                // "(效果已翻倍)", or read as "(效已倍)", sometimes on one line with the text under it (whose
                // brackets `normalise` leaves out)
                card.doubled = true;
                if normalise(&p.text).chars().count() >= 4 {
                    body.push(p);
                }
            } else {
                body.push(p);
            }
        }
        if body.is_empty() && title.is_none() {
            // a lone "確認" or stray marks: not a card
            continue;
        }
        let span: Vec<&Piece> = if body.is_empty() { col.clone() } else { body.clone() };
        let (x0, x1) = (span.iter().map(|p| p.x0).fold(f32::MAX, f32::min), span.iter().map(|p| p.x1).fold(f32::MIN, f32::max));
        card.x = title.map_or((x0 + x1) / 2.0, |t| t.cx());
        let half = title.map_or(0.022 * width, |t| (t.x1 - t.x0) / 2.0);
        card.title[0] = card.x - half;
        card.title[2] = card.x + half;
        // a neighbour's line can reach under this card (small text, long lines nearly touching): read without the
        // parts off its middle when the whole doesn't make a seal
        let near: Vec<&Piece> = body.iter().copied().filter(|p| (p.cx() - card.x).abs() <= 0.3 * apart || (p.x0 <= card.x && p.x1 >= card.x)).collect();
        let whole = reading_order(body.clone());
        card.text = if near.len() < body.len() && identify(card.category, &whole, None, card.doubled).is_none() {
            let core = reading_order(near);
            if identify(card.category, &core, None, card.doubled).is_some() { core } else { whole }
        } else {
            whole
        };
        out.push(card);
    }
    out
}

/// Pieces' text in reading order: row by row (pieces whose middles are within half a line of each other share
/// one, however their tops differ: "虛海寶藏的 掉落稀有度 +25%" comes in three pieces up to 3 pixels apart), each
/// row left to right.
fn reading_order(mut ps: Vec<&Piece>) -> String {
    ps.sort_by(|a, b| (a.y0 + a.y1).total_cmp(&(b.y0 + b.y1)));
    let mut rows: Vec<Vec<&Piece>> = Vec::new();
    for p in ps {
        let mid = (p.y0 + p.y1) / 2.0;
        match rows.last_mut() {
            Some(r) if (mid - (r[0].y0 + r[0].y1) / 2.0).abs() <= 0.5 * (r[0].y1 - r[0].y0).max(p.y1 - p.y0) => r.push(p),
            _ => rows.push(vec![p]),
        }
    }
    rows.into_iter()
        .map(|mut r| {
            r.sort_by(|a, b| a.x0.total_cmp(&b.x0));
            r.iter().map(|p| p.text.as_str()).collect::<String>()
        })
        .collect()
}

/// The rarity a card's title is drawn in (1 white, 2 blue, 3 purple, 4 gold): the mean colour of its brightest
/// pixels. Titles in the first test: white (137,157,167), blue (60,120,240), purple (170,80,190), gold (245,220,95).
pub fn title_rarity(img: &Img, card: &Card) -> Option<u8> {
    let [x0, y0, x1, y1] = card.title;
    let (x0, x1) = ((x0.max(0.0) as usize).min(img.width), (x1.max(0.0) as usize).min(img.width));
    let (y0, y1) = ((y0.max(0.0) as usize).min(img.height), (y1.max(0.0) as usize).min(img.height));
    let px: Vec<[f32; 3]> = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (y * img.width + x) * 4)).map(|i| [img.data[i] as f32, img.data[i + 1] as f32, img.data[i + 2] as f32]).collect();
    let lum = |p: &[f32; 3]| p[0].max(p[1]).max(p[2]);
    let (lo, hi) = px.iter().fold((255f32, 0f32), |(lo, hi), p| (lo.min(lum(p)), hi.max(lum(p))));
    if px.is_empty() || hi < 80.0 {
        return None;
    }
    let bright: Vec<&[f32; 3]> = px.iter().filter(|p| lum(p) >= lo + (hi - lo) * 0.6).collect();
    let n = bright.len() as f32;
    let [r, g, b] = bright.iter().fold([0f32; 3], |a, p| [a[0] + p[0] / n, a[1] + p[1] / n, a[2] + p[2] / n]);
    let spread = r.max(g).max(b) - r.min(g).min(b);
    Some(if spread < 60.0 {
        1
    } else if r > 180.0 && g > 140.0 && b < g {
        4
    } else if r > g + 40.0 && b > g + 40.0 {
        3
    } else if b > r + 60.0 {
        2
    } else {
        return None;
    })
}

/// The card picked by the light under it (its "確認" button and the light above it). Where to look is measured in
/// the titles' height, so it follows the interface's scale and the screen's size alike: 4 to 9 title heights under
/// the title, 2 either side (the button sits 3-6 under; a doubled card's text reaches the top of that, which at
/// 3840×2160 took 3.3-5.5's lit card's lead down to 1.1×). Over all 67 pictures of the tests (2560×1600,
/// 2560×1440, 1920×1080, 3840×2160, smallest scale to recommended) the brightest column is the one picked, at 1.32
/// to 3.8 times the next. None when no column clearly glows.
pub fn glowing(img: &Img, cards: &[Card]) -> Option<usize> {
    let (w, h) = (img.width as f32, img.height as f32);
    let glow: Vec<f32> = cards
        .iter()
        .map(|c| {
            let th = (c.title[3] - c.title[1]).max(8.0);
            let (x0, x1) = (((c.x - 2.0 * th).max(0.0) as usize), ((c.x + 2.0 * th).min(w) as usize));
            let (y0, y1) = (((c.title[3] + 4.0 * th).clamp(0.0, h) as usize), ((c.title[3] + 9.0 * th).clamp(0.0, h) as usize));
            let mut sum = 0f32;
            let mut n = 0f32;
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y * img.width + x) * 4;
                    sum += (img.data[i] as f32 + img.data[i + 1] as f32 + img.data[i + 2] as f32) / 3.0;
                    n += 1.0;
                }
            }
            if n > 0.0 { sum / n } else { 0.0 }
        })
        .collect();
    let best = (0..glow.len()).max_by(|a, b| glow[*a].total_cmp(&glow[*b]))?;
    let others = glow.iter().enumerate().filter(|(i, _)| *i != best).map(|(_, g)| *g).fold(0f32, f32::max);
    (glow[best] >= 45.0 && glow[best] >= 1.25 * others.max(1.0)).then_some(best)
}

/// A card as read over the screen's readings: the best of them.
#[derive(Debug, Clone)]
pub struct Column {
    pub x: f32,
    pub category: Option<&'static str>,
    /// the text of the reading that matched a seal, else the longest read
    pub text: String,
    pub matched: bool,
    pub doubled: bool,
    /// from its title's colour
    pub rarity: Option<u8>,
    /// how many readings it was in
    pub seen: u32,
}

/// The seal screen over its readings.
#[derive(Debug, Default, Clone)]
pub struct Board {
    pub columns: Vec<Column>,
    /// where the card last seen lit is
    pub lit: Option<f32>,
}

impl Board {
    /// One reading of the screen (`width` wide): its cards, their titles' rarities, the one lit if any.
    pub fn add(&mut self, cards: &[Card], rarities: &[Option<u8>], lit: Option<usize>, width: f32) {
        for (c, r) in cards.iter().zip(rarities) {
            let i = match self.columns.iter().position(|k| (k.x - c.x).abs() <= 0.07 * width) {
                Some(i) => i,
                None => {
                    self.columns.push(Column { x: c.x, category: None, text: String::new(), matched: false, doubled: false, rarity: None, seen: 0 });
                    self.columns.len() - 1
                }
            };
            let k = &mut self.columns[i];
            k.seen += 1;
            k.category = k.category.or(c.category);
            k.doubled |= c.doubled;
            k.rarity = k.rarity.or(*r);
            let matched = c.seal(*r).is_some();
            if (matched && !k.matched) || (matched == k.matched && c.text.chars().count() > k.text.chars().count()) {
                k.text = c.text.clone();
                k.matched = matched;
                if c.category.is_some() {
                    k.x = c.x;
                }
            }
        }
        self.columns.sort_by(|a, b| a.x.total_cmp(&b.x));
        if let Some(i) = lit {
            self.lit = cards.get(i).map(|c| c.x);
        }
    }

    /// The cards themselves: columns read at least a third as often as the most read one. A column read once or
    /// twice is the cards still sliding in, or stray words.
    pub fn cards(&self) -> Vec<&Column> {
        let most = self.columns.iter().map(|k| k.seen).max().unwrap_or(0);
        self.columns.iter().filter(|k| k.seen * 3 >= most).collect()
    }

    /// The column of the seal confirmed (of rarity `rarity`, from the log): the one last lit, else the only one of
    /// that rarity, else the only one.
    pub fn picked(&self, rarity: u8, width: f32) -> Option<&Column> {
        let cards = self.cards();
        if let Some(x) = self.lit {
            if let Some(k) = cards.iter().copied().filter(|k| (k.x - x).abs() <= 0.07 * width).min_by(|a, b| (a.x - x).abs().total_cmp(&(b.x - x).abs())) {
                return Some(k);
            }
        }
        let of: Vec<&Column> = cards.iter().copied().filter(|k| k.rarity == Some(rarity)).collect();
        if of.len() == 1 {
            return Some(of[0]);
        }
        (cards.len() == 1).then(|| cards[0])
    }
}

impl Column {
    /// The seal it is, of the rarity confirmed.
    pub fn seal(&self, rarity: Option<u8>) -> Option<&'static Seal> {
        identify(self.category, &self.text, rarity.or(self.rarity), self.doubled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line as Windows reads Chinese: a word per character or number.
    fn line(text: &str, x: f32, y: f32, ch: f32) -> Line {
        let mut out = Vec::new();
        let mut cx = x;
        for c in text.chars() {
            if c == ' ' {
                cx += ch * 0.3;
                continue;
            }
            out.push(Word { text: c.to_string(), x: cx, y, w: ch * 0.9, h: ch });
            cx += ch;
        }
        out
    }

    /// The first test's second pick (2560×1600): blue 財富, purple 軍械, gold 奇珍 picked, all doubled.
    fn second_pick() -> Vec<Line> {
        vec![
            {
                // one line across all three titles
                let mut l = line("財富秘語", 780.0, 920.0, 26.0);
                l.extend(line("軍械秘語", 1228.0, 920.0, 26.0));
                l.extend(line("奇珍秘語", 1690.0, 920.0, 26.0));
                l
            },
            line("(效果已翻倍)", 780.0, 980.0, 18.0),
            line("(效果已翻倍)", 1228.0, 992.0, 18.0),
            line("(效果已翻倍)", 1690.0, 992.0, 18.0),
            line("虛海寶藏基礎掉落中，掉落追憶碎絮的機率提升", 632.0, 1005.0, 18.0),
            line("42%", 808.0, 1028.0, 18.0),
            line("虛海寶藏額外掉落 8 枚英雄追憶", 1145.0, 1016.0, 18.0),
            line("虛海寶藏掉落的追憶遊絲 +84% 機率變成 3 倍", 1546.0, 1016.0, 18.0),
            line("確認", 1700.0, 1100.0, 40.0),
            line("請選擇一條封印秘語", 1190.0, 1180.0, 18.0),
        ]
    }

    #[test]
    fn three_cards() {
        let cs = cards(&second_pick(), 2560.0, 1600.0);
        assert_eq!(cs.iter().map(|c| c.category).collect::<Vec<_>>(), vec![Some("財富"), Some("軍械"), Some("奇珍")]);
        assert!(cs.iter().all(|c| c.doubled));
        assert_eq!(cs.iter().map(|c| c.chosen).collect::<Vec<_>>(), vec![false, false, true]);
        assert_eq!(cs[0].seal(None).unwrap().text, "虛海寶藏基礎掉落中，掉落追憶碎絮的機率提升 42%");
        assert_eq!(cs[1].seal(None).unwrap().text, "虛海寶藏額外掉落 8 枚英雄追憶");
        let gold = cs[2].seal(Some(4)).unwrap();
        assert_eq!(gold.text, "虛海寶藏掉落的追憶遊絲 +84% 機率變成 3 倍");
        assert_eq!((gold.rarity, gold.doubled), (4, true));
    }

    #[test]
    fn a_title_unread() {
        // the middle (white) card's title missed, and its text read on one line with its neighbours'
        let mut lines = vec![{
            let mut l = line("財富秘語", 780.0, 920.0, 26.0);
            l.extend(line("奇珍秘語", 1690.0, 920.0, 26.0));
            l
        }];
        let mut l = line("虛海寶藏基礎掉落中，追憶碎絮的掉落數量額外", 632.0, 992.0, 18.0);
        l.extend(line("虛海寶藏基礎掉落中，追憶碎絮的掉落數量額外", 1082.0, 994.0, 18.0));
        l.extend(line("虛海寶藏的掉落數量 + 14%", 1626.0, 1004.0, 18.0));
        lines.push(l);
        lines.push(line("+ 53%", 800.0, 1015.0, 18.0));
        lines.push(line("+35%", 1251.0, 1016.0, 18.0));
        let cs = cards(&lines, 2560.0, 1600.0);
        assert_eq!(cs.len(), 3);
        assert_eq!(cs[1].category, None);
        assert_eq!(cs[1].seal(Some(1)).unwrap().text, "虛海寶藏基礎掉落中，追憶碎絮的掉落數量額外 + 35%");
        assert_eq!(cs[0].seal(Some(2)).unwrap().text, "虛海寶藏基礎掉落中，追憶碎絮的掉落數量額外 + 53%");
        // the middle card's title would be on the others' row
        assert!((cs[1].title[1] - 920.0).abs() < 1.0);
    }

    #[test]
    fn a_neighbours_line_under_a_card() {
        // 1920×1080 at the smallest scale (the ninth test): the middle card's long line reaches under the right
        // card, and its end is taken for part of it
        let mut titles = line("軍械秘語", 600.0, 629.0, 15.0);
        titles.extend(line("軍械秘語", 936.0, 629.0, 15.0));
        titles.extend(line("奇珍秘語", 1283.0, 629.0, 15.0));
        let mut body = line("虛海寶藏有 20% 機率額外掉落", 812.0, 695.0, 11.0);
        body.extend(line("外掉 12 枚英", 1118.0, 695.0, 11.0));
        body.extend(line("虍藏的掉落數量*4%", 1235.0, 695.0, 11.0));
        let cs = cards(&[titles, body], 1920.0, 1080.0);
        assert_eq!(cs.len(), 3);
        assert_eq!(cs[2].seal(Some(1)).unwrap().text, "虛海寶藏的掉落數量 + 14%");
    }

    #[test]
    fn the_base_card_alone() {
        let lines = vec![line("奇珍秘語", 1230.0, 1000.0, 26.0), line("(效果已翻倍)", 1210.0, 1060.0, 18.0), line("虛海寶藏的基礎掉落數量 +18%", 1150.0, 1085.0, 18.0)];
        let cs = cards(&lines, 2560.0, 1600.0);
        assert_eq!(cs.len(), 1);
        let s = cs[0].seal(Some(1)).unwrap();
        assert_eq!((s.numbers[0], s.rarity, s.doubled), (18, 1, true));
    }

    #[test]
    fn readings_kept_at_their_best() {
        let good = cards(&second_pick(), 2560.0, 1600.0);
        // a later reading: the middle card's text half read
        let mut bad = good.clone();
        bad[1].text = "虛海寶藏額外".into();
        let mut b = Board::default();
        b.add(&good, &[Some(2), Some(3), Some(4)], None, 2560.0);
        b.add(&bad, &[Some(2), Some(3), Some(4)], Some(2), 2560.0);
        assert_eq!(b.columns.len(), 3);
        assert_eq!(b.columns[1].seal(Some(3)).unwrap().text, "虛海寶藏額外掉落 8 枚英雄追憶");
        assert_eq!(b.picked(4, 2560.0).unwrap().category, Some("奇珍"));
        // nothing lit: the rarity decides when it's the only one of its colour
        let mut c = Board::default();
        c.add(&good, &[Some(1), Some(1), Some(4)], None, 2560.0);
        assert_eq!(c.picked(4, 2560.0).unwrap().category, Some("奇珍"));
        assert!(c.picked(1, 2560.0).is_none());
        // a column read once while the cards slid in doesn't count
        let mut d = Board::default();
        let mut sliding = good.clone();
        sliding[1].x += 231.0;
        sliding[1].text = "額外".into();
        d.add(&sliding[1..2], &[Some(3)], None, 2560.0);
        for _ in 0..4 {
            d.add(&good, &[Some(2), Some(3), Some(4)], None, 2560.0);
        }
        assert_eq!(d.columns.len(), 4);
        assert_eq!(d.cards().len(), 3);
    }
}
