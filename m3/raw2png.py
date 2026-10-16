import sys
from PIL import Image
frame = open(sys.argv[1], "rb").read()
pal = open(sys.argv[2], "rb").read()
im = Image.frombytes("P", (320, 200), frame)
flat = []
for i in range(256):
    r, g, b = pal[i * 3 : i * 3 + 3] if i < 18 else (0, 0, 0)
    flat += [min(255, r * 255 // 63), min(255, g * 255 // 63), min(255, b * 255 // 63)]
im.putpalette(flat)
im = im.convert("RGB").resize((960, 600), Image.NEAREST)
im.save(sys.argv[3])
print("wrote", sys.argv[3])
