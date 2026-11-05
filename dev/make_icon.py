"""Draws SmartTorch's icon (a flame with an S cut out of it) into crates/vorax-overlay/assets/vorax.ico and
vorax.png: each size drawn on its own, the small ones with a bolder S so it still reads at 16 px.

    pip install --break-system-packages cairosvg pillow
    python3 dev/make_icon.py <repo root>
"""
import io, os, sys
import cairosvg
from PIL import Image

root = sys.argv[1] if len(sys.argv) > 1 else '.'
FLAME = "M32 6 C40 16 50 22 50 36 C50 47 42 56 32 56 C22 56 14 47 14 36 C14 27 20 22 23 15 C25 22 28 24 31 24 C28 18 29 11 32 6 Z"
S = "M38.5 31.5 C37 29 34.5 28 32 28 C28.5 28 26 30 26 33 C26 39 38.5 37 38.5 43.5 C38.5 46.5 35.5 48.5 32 48.5 C29 48.5 26.5 47 25 44.5"


def svg(size):
    big = size >= 48
    fill = 'url(#f)' if big else '#f2953f'
    stroke = 5 if big else (6 if size >= 32 else 7)
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 64 64">
<defs><linearGradient id="f" x1="0" y1="1" x2="0" y2="0"><stop offset="0" stop-color="#e05a24"/><stop offset="1" stop-color="#ffcf6b"/></linearGradient></defs>
<rect width="64" height="64" rx="14" fill="#1b1e25"/><path d="{FLAME}" fill="{fill}"/>
<path d="{S}" fill="none" stroke="#1b1e25" stroke-width="{stroke}" stroke-linecap="round"/></svg>'''


def png(size):
    return Image.open(io.BytesIO(cairosvg.svg2png(bytestring=svg(size).encode(), output_width=size, output_height=size))).convert('RGBA')


sizes = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]
images = [png(s) for s in sizes]
assets = os.path.join(root, 'crates/vorax-overlay/assets')
images[-1].save(os.path.join(assets, 'vorax.ico'), format='ICO', sizes=[(s, s) for s in sizes], append_images=images[:-1])
images[-1].save(os.path.join(assets, 'vorax.png'))
print('vorax.ico', sorted(Image.open(os.path.join(assets, 'vorax.ico')).ico.sizes()))
# a sheet of every size, to look at
sheet = Image.new('RGBA', (sum(sizes) + 12 * len(sizes) + 12, 256 + 24), (15, 17, 21, 255))
x = 12
for s, im in zip(sizes, images):
    sheet.paste(im, (x, 12 + 256 - s), im)
    x += s + 12
sheet.save('/tmp/icon-sheet.png')
