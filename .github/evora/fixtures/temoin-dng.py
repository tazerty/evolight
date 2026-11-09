#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Evora — DNG SYNTHÉTIQUE du témoin RAW de la fumée (EVORA-REV-73).

L'image est entièrement calculée ici, en entiers : aucune photo, aucune image tierce. Quatre bandes de
384 × 64 en valeurs LINÉAIRES sur 12 bits : un dégradé de gris, un dégradé de teintes, huit aplats
saturés, huit aplats de gris. Mosaïque de Bayer RGGB, échantillons de 16 bits non compressés, une seule
IFD TIFF/EP portant les étiquettes publiques du DNG (structure reprise de l'outil jpeg-vers-dng.py
d'Evora, montée v0.19.0). Espace caméra = sRGB linéaire : ColorMatrix1 = XYZ (D65) vers sRGB,
AsShotNeutral 1, 1, 1. Le calcul n'emploie que des entiers : le fichier est le même, octet pour octet,
sur toute machine, et la CI vérifie que le fichier versionné est bien celui-ci.

Usage :
  temoin-dng.py write <sortie.dng>   écrit le fichier (refuse d'écraser un fichier existant)
  temoin-dng.py check <fichier.dng>  rouge si le fichier n'est pas exactement celui que produit ce script
"""
import hashlib
import struct
import sys

WIDTH, HEIGHT, BAND = 384, 256, 64
WHITE = 4095  # niveau blanc sur 12 bits (valeurs rangées sur 16 bits)
SATURATED = [(230, 40, 40), (40, 200, 60), (40, 60, 220), (40, 200, 210),
             (210, 50, 200), (230, 210, 40), (240, 140, 30), (110, 50, 170)]


def linear(c):
    """0..255 vers une valeur linéaire de 12 bits, loin du noir et du blanc (ni écrêtée, ni nulle)."""
    return 150 + c * 3200 // 255


def hue(x):
    """Teinte entière le long de la largeur : six segments de 256 pas, du rouge au rouge."""
    h = x * 1536 // WIDTH
    seg, f = divmod(h, 256)
    return [(255, f, 0), (255 - f, 255, 0), (0, 255, f), (0, 255 - f, 255), (f, 0, 255), (255, 0, 255 - f)][seg]


def pixel(x, y):
    band = y // BAND
    if band == 0:
        v = 40 + x * 3900 // (WIDTH - 1)
        return v, v, v
    if band == 1:
        return tuple(linear(c) for c in hue(x))
    k = x * 8 // WIDTH
    if band == 2:
        return tuple(linear(c) for c in SATURATED[k])
    v = 60 + k * 500
    return v, v, v


def mosaic():
    out = bytearray(WIDTH * HEIGHT * 2)
    for y in range(HEIGHT):
        for x in range(WIDTH):
            r, g, b = pixel(x, y)
            v = (r if x % 2 == 0 else g) if y % 2 == 0 else (g if x % 2 == 0 else b)
            struct.pack_into("<H", out, (y * WIDTH + x) * 2, min(v, WHITE))
    return bytes(out)


def build(data):
    # (étiquette, type, valeurs) ; types : 1 BYTE, 2 ASCII, 3 SHORT, 4 LONG, 5 RATIONAL, 10 SRATIONAL
    xyz_to_srgb = [(32406, 10000), (-15372, 10000), (-4986, 10000), (-9689, 10000), (18758, 10000),
                   (415, 10000), (557, 10000), (-2040, 10000), (10570, 10000)]
    entries = [
        (254, 4, [0]),                          # NewSubFileType : image principale
        (256, 4, [WIDTH]),                      # ImageWidth
        (257, 4, [HEIGHT]),                     # ImageLength
        (258, 3, [16]),                         # BitsPerSample
        (259, 3, [1]),                          # Compression : aucune
        (262, 3, [32803]),                      # PhotometricInterpretation : CFA
        (271, 2, b"Evora\x00"),                 # Make
        (272, 2, b"Temoin RAW EVORA-REV-73\x00"),  # Model
        (273, 4, [0]),                          # StripOffsets (posé plus bas)
        (274, 3, [1]),                          # Orientation
        (277, 3, [1]),                          # SamplesPerPixel
        (278, 4, [HEIGHT]),                     # RowsPerStrip
        (279, 4, [len(data)]),                  # StripByteCounts
        (284, 3, [1]),                          # PlanarConfiguration
        (33421, 3, [2, 2]),                     # CFARepeatPatternDim
        (33422, 1, bytes([0, 1, 1, 2])),        # CFAPattern : RGGB
        (50706, 1, bytes([1, 4, 0, 0])),        # DNGVersion 1.4
        (50707, 1, bytes([1, 1, 0, 0])),        # DNGBackwardVersion 1.1
        (50708, 2, b"Evora Temoin RGGB\x00"),   # UniqueCameraModel
        (50714, 3, [0]),                        # BlackLevel
        (50717, 3, [WHITE]),                    # WhiteLevel
        (50721, 10, xyz_to_srgb),               # ColorMatrix1
        (50728, 5, [(1, 1), (1, 1), (1, 1)]),   # AsShotNeutral
        (50778, 3, [21]),                       # CalibrationIlluminant1 : D65
    ]
    entries.sort(key=lambda e: e[0])
    packers = {3: "<H", 4: "<I", 5: "<II", 10: "<ii"}
    ifd_off = 8
    extra_off = ifd_off + 2 + 12 * len(entries) + 4
    extra = bytearray()
    placed = []
    for tag, typ, vals in entries:
        if typ in (1, 2):
            payload, count = bytes(vals), len(vals)
        else:
            fmt = packers[typ]
            payload = b"".join(struct.pack(fmt, *v) if isinstance(v, tuple) else struct.pack(fmt, v) for v in vals)
            count = len(vals)
        if len(payload) > 4:  # hors de l'IFD, aligné sur 2
            if len(extra) % 2:
                extra += b"\x00"
            placed.append((tag, typ, count, None, extra_off + len(extra)))
            extra += payload
        else:
            placed.append((tag, typ, count, payload.ljust(4, b"\x00"), None))
    data_off = extra_off + len(extra)
    data_off += data_off % 2
    out = bytearray(b"II*\x00" + struct.pack("<I", ifd_off))
    out += struct.pack("<H", len(placed))
    for tag, typ, count, inline, off in placed:
        if tag == 273:
            inline = struct.pack("<I", data_off)
        out += struct.pack("<HHI", tag, typ, count)
        out += inline if inline is not None else struct.pack("<I", off)
    out += struct.pack("<I", 0)
    out += extra
    out += b"\x00" * (data_off - len(out))
    out += data
    return bytes(out)


def main(argv):
    if len(argv) != 2 or argv[0] not in ("write", "check"):
        print(__doc__)
        return 2
    dng = build(mosaic())
    digest = hashlib.sha256(dng).hexdigest()
    if argv[0] == "write":
        with open(argv[1], "xb") as f:
            f.write(dng)
        print(f"DNG {WIDTH}x{HEIGHT} RGGB, {len(dng)} octets, sha256 {digest} : {argv[1]}")
        return 0
    try:
        with open(argv[1], "rb") as f:
            kept = f.read()
    except OSError as err:
        print(f"::error::témoin RAW illisible : {err}")
        return 1
    if kept != dng:
        print(f"::error::{argv[1]} n'est pas le DNG que produit temoin-dng.py (sha256 {hashlib.sha256(kept).hexdigest()} au lieu de {digest})")
        return 1
    print(f"Témoin RAW : {argv[1]} est le DNG du script ({len(dng)} octets, sha256 {digest}).")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
