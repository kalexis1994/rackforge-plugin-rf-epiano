"""Generate the RF-EPiano package artwork (Pillow 12+)."""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

TOOLS = Path(__file__).resolve().parent
OUT = TOOLS.parent / "package" / "branding"
OUT.mkdir(parents=True, exist_ok=True)
IVORY = "#f3ead4"
MINT = "#acd2b7"
SEMI_BOLD = TOOLS / "fonts" / "BarlowSemiCondensed-SemiBold.ttf"
BOLD = TOOLS / "fonts" / "BarlowSemiCondensed-Bold.ttf"


def background(width, height):
    image = Image.new("RGB", (width, height))
    draw = ImageDraw.Draw(image)
    for y in range(height):
        light = 1 - y / height
        color = (round(13 + 14 * light), round(28 + 32 * light), round(22 + 24 * light))
        draw.line((0, y, width, y), fill=color)
    return image, draw


def text(draw, point, value, size, fill=IVORY, weight=SEMI_BOLD):
    draw.text(point, value, fill=fill, font=ImageFont.truetype(weight, size), anchor="lt")


def instrument(draw, x, y, width, height, keys=21):
    border = round(height * .06)
    draw.rounded_rectangle((x, y, x + width, y + height), radius=border * 2,
                           fill="#184a36", outline="#75a58b", width=2)
    draw.rounded_rectangle((x + border, y + border, x + width - border, y + height - border),
                           radius=border, fill="#122e23")
    head_top = y + border + 2
    head_bottom = y + round(height * .46)
    metal_left, metal_right = x + round(width * .24), x + round(width * .76)
    draw.rounded_rectangle((metal_left, head_top, metal_right, head_bottom), radius=4,
                           fill="#d9ddce", outline="#eff2e7", width=2)
    for a, b in [(x + border + 5, metal_left - 8), (metal_right + 8, x + width - border - 5)]:
        draw.rounded_rectangle((a, head_top, b, head_bottom), radius=5, fill="#25543e")
        spacing = max(4, border // 2)
        for row in range(head_top + 4, head_bottom - 3, spacing):
            for col in range(a + 4, b - 3, spacing):
                draw.ellipse((col, row, col + 1, row + 1), fill="#0a2017")
    radius = max(3, (head_bottom - head_top) // 5)
    for i in range(3):
        cx = metal_left + round((metal_right - metal_left) * (.25 + i * .2))
        cy = (head_top + head_bottom) // 2
        draw.ellipse((cx - radius, cy - radius, cx + radius, cy + radius),
                     fill="#28362e", outline="#f5f3e5", width=2)
        draw.line((cx, cy - radius + 3, cx, cy - max(1, radius // 3)), fill=IVORY, width=2)

    key_top = y + round(height * .54)
    key_bottom = y + height - border
    left, right = x + border + 2, x + width - border - 2
    pitch = (right - left) / keys
    for i in range(keys):
        a, b = int(left + i * pitch), int(left + (i + 1) * pitch) - 2
        draw.rectangle((a, key_top, b, key_bottom), fill="#c6b99c")
        draw.rectangle((a, key_top, b, key_bottom - max(2, border // 3)), fill=IVORY)
    for i in range(keys - 1):
        if i % 7 in (0, 1, 3, 4, 5):
            at = left + (i + 1) * pitch
            draw.rounded_rectangle((round(at - pitch * .31), key_top, round(at + pitch * .31),
                                    round(key_top + (key_bottom - key_top) * .63)),
                                   radius=max(1, border // 6), fill="#1a231e", outline="#091611")


def line(draw, x, y, width):
    draw.line((x, y, x + width, y), fill="#5a8a70", width=2)


im, draw = background(512, 512)
draw.rounded_rectangle((50, 50, 462, 462), radius=36, outline="#6b987d", width=3)
text(draw, (82, 95), "RF-EPiano", 73, weight=BOLD)
text(draw, (85, 181), "REED SERIES / 200A", 27, MINT)
instrument(draw, 82, 237, 348, 133, 14)
text(draw, (85, 402), "RACKFORGE INSTRUMENTS", 23, MINT)
im.save(OUT / "icon.png", optimize=True)

im, draw = background(1600, 400)
line(draw, 248, 319, 1104)
text(draw, (248, 79), "RACKFORGE INSTRUMENTS", 26, MINT)
text(draw, (242, 127), "RF-EPiano", 99, weight=BOLD)
text(draw, (249, 252), "REED SERIES / 200A", 31, MINT)
instrument(draw, 849, 111, 488, 174)
im.save(OUT / "banner.png", optimize=True)

im, draw = background(1920, 1080)
line(draw, 224, 852, 1472)
text(draw, (224, 224), "RACKFORGE INSTRUMENTS", 39, MINT)
text(draw, (214, 303), "RF-EPiano", 164, weight=BOLD)
text(draw, (226, 512), "REED SERIES / 200A", 47, MINT)
instrument(draw, 939, 485, 756, 255)
text(draw, (227, 792), "Powered by OpenWurli", 31, MINT)
im.save(OUT / "splash.png", optimize=True)
