"""Builds crates/vorax-overlay/ui/t2s.json: every CJK character that Simplified Chinese writes differently, and how
(OpenCC t2s, one character at a time). The toolbox's main window (ui/index.html) shows its words in Simplified with
it, release notes included, so it covers all characters, not only the page's.

    pip install --break-system-packages opencc-python-reimplemented
    python3 dev/make_ui_t2s.py <repo root>
"""
import json, os, sys
import opencc

root = sys.argv[1] if len(sys.argv) > 1 else '.'
t2s = opencc.OpenCC('t2s')
table = {}
for cp in list(range(0x3400, 0x4DC0)) + list(range(0x4E00, 0xA000)) + list(range(0xF900, 0xFB00)):
    c = chr(cp)
    s = t2s.convert(c)
    if len(s) == 1 and s != c:
        table[c] = s
out = os.path.join(root, 'crates/vorax-overlay/ui/t2s.json')
json.dump(table, open(out, 'w', encoding='utf-8'), ensure_ascii=False, separators=(',', ':'), sort_keys=True)
print(out, len(table), 'characters,', os.path.getsize(out), 'bytes')
