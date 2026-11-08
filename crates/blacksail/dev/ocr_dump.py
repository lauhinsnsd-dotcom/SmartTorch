# RapidOCR over a seal screen's band (as the recorder reads it), lines as TSV: text, x, y, w, h (screen pixels)
import sys
from PIL import Image
from rapidocr_onnxruntime import RapidOCR
eng = RapidOCR()
src, dst = sys.argv[1], sys.argv[2]
im = Image.open(src).convert('RGB')
W, H = im.size
x0, y0, x1, y1 = int(W*0.08), int(H*0.40), int(W*0.92), int(H*0.85)
band = im.crop((x0, y0, x1, y1))
import numpy as np
res, _ = eng(np.asarray(band)[:, :, ::-1].copy())
with open(dst, 'w', encoding='utf-8') as f:
    f.write(f"# {W}x{H}\n")
    for box, text, score in res or []:
        xs = [p[0] for p in box]; ys = [p[1] for p in box]
        f.write(f"{text}\t{min(xs)+x0:.0f}\t{min(ys)+y0:.0f}\t{max(xs)-min(xs):.0f}\t{max(ys)-min(ys):.0f}\n")
