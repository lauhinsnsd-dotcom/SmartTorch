"""A stand-in for Windows' Chinese OCR, for testing the title reader on Linux (see dev_cli_helpers.rs).

    pip install --break-system-packages rapidocr_onnxruntime
    python3 dev/rocr.py crop.png
"""
import sys
from rapidocr_onnxruntime import RapidOCR
r, _ = RapidOCR()(sys.argv[1])
print("\n".join(x[1] for x in (r or [])))
