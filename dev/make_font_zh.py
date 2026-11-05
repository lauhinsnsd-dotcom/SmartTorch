"""Builds the Chinese edition's built-in fonts and its Traditional-to-Simplified table, cut to the characters the
overlay can show: every card name in names_zh.json, every Chinese string in crates/vorax-overlay/src, and printable
ASCII. Windows' own fonts (Segoe UI) have no Chinese. (The main window, ui/index.html, uses Windows' own Chinese
fonts and dev/make_ui_t2s.py's table instead.)

    data/fonts/zh-bold.ttf, zh-regular.ttf      Noto Sans TC (SIL OFL 1.1), the Traditional characters
    data/fonts/sc-bold.ttf, sc-regular.ttf      Noto Sans SC (SIL OFL 1.1), their Simplified forms
    data/fonts/t2s.json                         each character that Simplified writes differently, and how (OpenCC)

    pip install --break-system-packages fonttools opencc-python-reimplemented
    python3 make_font_zh.py NotoSansTC[wght].ttf NotoSansSC[wght].ttf <repo root>

The two fonts: https://github.com/google/fonts/tree/main/ofl/notosanstc and .../notosanssc (their OFL.txt is
the same; it sits next to the fonts). Run again whenever names_zh.json or the overlay's Chinese words change:
`cargo test` fails while a character is missing.
"""
import glob, json, os, re, sys
import opencc
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
from fontTools.subset import Subsetter, Options

tc_src, sc_src, root = sys.argv[1], sys.argv[2], sys.argv[3]
cjk = re.compile(r'[　-〿㐀-鿿豈-﫿＀-￯]')
names = json.load(open(os.path.join(root, 'data/scan/names_zh.json'), encoding='utf-8'))
chars = set(chr(c) for c in range(0x20, 0x7f))
for phase in ('potion', 'instrument', 'protocol'):
    for forms in names[phase].values():
        for f in forms:
            chars.update(f)
for n in names.get('unsupported', []):
    chars.update(n)
# every Chinese string in the overlay's source
for path in glob.glob(os.path.join(root, 'crates/vorax-overlay/src/**/*.rs'), recursive=True):
    for lit in re.findall(r'"((?:[^"\\]|\\.)*)"', open(path, encoding='utf-8').read()):
        if cjk.search(lit):
            chars.update(lit)
chars.update('→·×')

t2s = opencc.OpenCC('t2s')
table = {}
for c in sorted(chars):
    s = t2s.convert(c)
    if len(s) == 1 and s != c:
        table[c] = s

out_dir = os.path.join(root, 'data/fonts')
os.makedirs(out_dir, exist_ok=True)
json.dump(table, open(os.path.join(out_dir, 't2s.json'), 'w', encoding='utf-8'), ensure_ascii=False, indent=0, sort_keys=True)


def cut(src, keep, weight, name):
    font = instantiateVariableFont(TTFont(src), {'wght': weight})
    opts = Options()
    opts.layout_features = []
    opts.name_IDs = ['*']
    opts.notdef_outline = True
    sub = Subsetter(opts)
    sub.populate(unicodes=[ord(c) for c in keep])
    sub.subset(font)
    path = os.path.join(out_dir, name)
    font.save(path)
    print(name, len(keep), 'characters,', os.path.getsize(path), 'bytes')


for weight, suffix in ((700, 'bold'), (400, 'regular')):
    cut(tc_src, chars, weight, f'zh-{suffix}.ttf')
    # the Simplified font only needs the characters Simplified writes differently
    cut(sc_src, set(table.values()), weight, f'sc-{suffix}.ttf')
print('t2s.json', len(table), 'characters')
