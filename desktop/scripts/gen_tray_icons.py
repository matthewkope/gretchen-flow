# /// script
# dependencies = ["pillow"]
# ///
"""Generate menu-bar tray icons (44x44 for 22pt @2x retina).

- idle.png: regular-weight black ¿ rendered as a macOS template image
  (the OS recolors it for light/dark menu bars)
- recording.png: bold red ¿ with a glow — "live" indicator while the hotkey is held
- transcribing.png: bold amber ¿ — busy indicator while Whisper runs

Run: uv run desktop/scripts/gen_tray_icons.py
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

SIZE = 44
OUT = Path(__file__).parent.parent / "src-tauri" / "icons" / "tray"

FONT_PATH = "/System/Library/Fonts/Helvetica.ttc"


def find_font(size: int, bold: bool) -> ImageFont.FreeTypeFont:
    for index in range(6):
        try:
            font = ImageFont.truetype(FONT_PATH, size, index=index)
        except OSError:
            break
        name = " ".join(font.getname())
        if bold and "Bold" in name and "Oblique" not in name:
            return font
        if not bold and "Bold" not in name and "Oblique" not in name and "Light" not in name:
            return font
    raise SystemExit(f"no {'bold' if bold else 'regular'} face found in {FONT_PATH}")


def glyph(font: ImageFont.FreeTypeFont, color, stroke: int = 0, glow=None) -> Image.Image:
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    center = (SIZE / 2, SIZE / 2 - 1)
    if glow:
        layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
        ImageDraw.Draw(layer).text(
            center, "¿", font=font, fill=glow, anchor="mm", stroke_width=stroke + 2,
            stroke_fill=glow,
        )
        img.alpha_composite(layer.filter(ImageFilter.GaussianBlur(4)))
    ImageDraw.Draw(img).text(
        center, "¿", font=font, fill=color, anchor="mm", stroke_width=stroke, stroke_fill=color
    )
    return img


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    regular = find_font(34, bold=False)
    bold = find_font(36, bold=True)
    print(f"regular: {' '.join(regular.getname())}, bold: {' '.join(bold.getname())}")

    glyph(regular, (0, 0, 0, 255)).save(OUT / "idle.png")
    glyph(bold, (255, 59, 48, 255), stroke=1, glow=(255, 59, 48, 200)).save(
        OUT / "recording.png"
    )
    glyph(bold, (255, 159, 10, 255), stroke=1).save(OUT / "transcribing.png")
    print(f"wrote 3 icons to {OUT}")


if __name__ == "__main__":
    main()
