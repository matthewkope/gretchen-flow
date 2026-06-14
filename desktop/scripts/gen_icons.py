# /// script
# dependencies = ["pillow"]
# ///
"""Generate the app icon and menu-bar tray icons from the Gretchen artwork
(icons/gretchen-source.png, white line art on black).

- icons/icon.png: 1024x1024 app icon — white art on a rounded dark square
- icons/tray/idle.png: 44x44 plain white Gretchen outline on transparency (idle)
- icons/tray/recording.png: white Gretchen on a black rounded badge (recording)

Run: uv run desktop/scripts/gen_icons.py
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

ICONS = Path(__file__).parent.parent / "src-tauri" / "icons"
SOURCE = ICONS / "gretchen-source.png"
TRAY = ICONS / "tray"
TRAY_SIZE = 44


def luminance_mask(img: Image.Image) -> Image.Image:
    """White-on-black art -> grayscale mask (white lines become the glyph)."""
    return img.convert("L")


def app_icon(art: Image.Image) -> Image.Image:
    size = 1024
    icon = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(icon)
    draw.rounded_rectangle([64, 64, size - 64, size - 64], radius=200, fill=(24, 22, 34, 255))
    inner = size - 2 * 128
    mask = luminance_mask(art).resize((inner, inner), Image.LANCZOS)
    white = Image.new("RGBA", (inner, inner), (240, 238, 248, 255))
    icon.paste(white, (128, 128), mask)
    return icon


def badge_icon(art: Image.Image, top, bottom, fg=(255, 255, 255), boost: float = 1.6) -> Image.Image:
    """Line art in the foreground, vertical-gradient rounded badge behind
    (pass top == bottom for a solid color)."""
    size = TRAY_SIZE
    gradient = Image.new("RGBA", (size, size))
    for y in range(size):
        t = y / (size - 1)
        row = tuple(int(top[c] + (bottom[c] - top[c]) * t) for c in range(3)) + (255,)
        for x in range(size):
            gradient.putpixel((x, y), row)
    badge_mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(badge_mask).rounded_rectangle([1, 1, size - 2, size - 2], radius=10, fill=255)

    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    img.paste(gradient, (0, 0), badge_mask)

    art_mask = luminance_mask(art).resize((size, size), Image.LANCZOS)
    art_mask = art_mask.point(lambda v: min(255, int(v * boost)))
    fill = Image.new("RGBA", (size, size), fg + (255,))
    img.paste(fill, (0, 0), art_mask)
    return img


def tray_icon(art: Image.Image, color, glow=None, boost: float = 1.6) -> Image.Image:
    mask = luminance_mask(art).resize((TRAY_SIZE, TRAY_SIZE), Image.LANCZOS)
    # Boost so fine lines survive the heavy downscale.
    mask = mask.point(lambda v: min(255, int(v * boost)))
    img = Image.new("RGBA", (TRAY_SIZE, TRAY_SIZE), (0, 0, 0, 0))
    if glow:
        layer = Image.new("RGBA", (TRAY_SIZE, TRAY_SIZE), (0, 0, 0, 0))
        fill = Image.new("RGBA", (TRAY_SIZE, TRAY_SIZE), glow)
        layer.paste(fill, (0, 0), mask)
        img.alpha_composite(layer.filter(ImageFilter.GaussianBlur(3)))
    fill = Image.new("RGBA", (TRAY_SIZE, TRAY_SIZE), color)
    img.paste(fill, (0, 0), mask)
    return img


def main() -> None:
    art = Image.open(SOURCE)
    TRAY.mkdir(parents=True, exist_ok=True)

    app_icon(art).save(ICONS / "icon.png")
    # Idle: plain white Gretchen outline. Recording: white Gretchen on black.
    black = (0, 0, 0)
    tray_icon(art, (255, 255, 255, 255)).save(TRAY / "idle.png")
    badge_icon(art, black, black).save(TRAY / "recording.png")
    print(f"wrote icon.png and 2 tray icons under {ICONS}")


if __name__ == "__main__":
    main()
