# /// script
# dependencies = ["pillow"]
# ///
"""Generate the app icon and menu-bar tray icons from the Gretchen artwork
(icons/gretchen-source.png, white line art on black).

- icons/icon.png: 1024x1024 app icon — white art on a rounded dark square
- icons/tray/idle.png: 44x44 template image (black + alpha from the art's
  luminance; macOS recolors it for light/dark menu bars)
- icons/tray/recording.png: red Gretchen with a glow — "live" indicator
- icons/tray/transcribing.png: amber Gretchen — busy indicator

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
    tray_icon(art, (0, 0, 0, 255)).save(TRAY / "idle.png")
    tray_icon(art, (255, 59, 48, 255), glow=(255, 59, 48, 190)).save(TRAY / "recording.png")
    tray_icon(art, (255, 159, 10, 255)).save(TRAY / "transcribing.png")
    print(f"wrote icon.png and 3 tray icons under {ICONS}")


if __name__ == "__main__":
    main()
