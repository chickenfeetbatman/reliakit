#!/usr/bin/env python3
"""Generate ReliaKit's original SVG icon set (stroke glyphs, 24x24 grid).

Every glyph is drawn from plain shapes, matching the icon concepts in
docs/DESIGN.md. Shapes are black; the UI tints them with Slint's
colorize property. No third-party icon art is used.
"""
import os

OUT = os.path.join(os.path.dirname(__file__), "..", "ui", "icons")
os.makedirs(OUT, exist_ok=True)

HEAD = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">'
TAIL = "</svg>"

ICONS = {
    # Item kinds
    "life": '<path d="M4 3v17h17"/><path d="M5 6c5 1 9 4 12 8 1.2 1.6 2 3 2 3"/>',
    "limited": '<path d="M4 5v14M4 5h3M4 19h3"/><circle cx="12" cy="8" r="1" fill="#000"/><circle cx="16.5" cy="12" r="1" fill="#000"/><circle cx="12" cy="16" r="1" fill="#000"/>',
    "mcmc": '<circle cx="6" cy="7" r="3"/><circle cx="17" cy="7" r="3"/><circle cx="11.5" cy="17" r="3"/><path d="M8.8 8.6l1 4.4M14.4 8.6l-1.2 4.4M9 7h5"/>',
    "degradation": '<path d="M4 4l16 13"/><path d="M3 15h18" stroke-dasharray="3 3"/><circle cx="8" cy="7.3" r="1" fill="#000"/><circle cx="14" cy="12.2" r="1" fill="#000"/>',
    "alt": '<path d="M12 4L21 20H3z"/><path d="M12.8 10l-2 3.4h2.4l-1.8 3.1" />',
    "growth": '<path d="M4 3v17h17"/><path d="M5 18c4-1 6-3 8-6 1.8-2.7 3-4.6 6-6"/>',
    "competing": '<path d="M3 12h6"/><path d="M9 12c3 0 4-5 7-5h5M9 12c3 0 4 5 7 5h5"/><circle cx="9" cy="12" r="1.2" fill="#000"/>',
    "rbd": '<rect x="2.5" y="9" width="7" height="6" rx="1"/><rect x="14.5" y="9" width="7" height="6" rx="1"/><path d="M9.5 12h5"/>',
    "faulttree": '<circle cx="12" cy="4.5" r="2"/><circle cx="6" cy="19" r="2"/><circle cx="18" cy="19" r="2"/><path d="M12 6.5v4M12 10.5L6.8 17M12 10.5l5.2 6.5"/>',
    "fmea": '<rect x="3" y="4" width="18" height="16" rx="1"/><path d="M3 9.3h18M3 14.6h18M9 4v16M15 4v16"/>',
    "warranty": '<path d="M5 6a9 9 0 1 1-1 7"/><path d="M3.5 3v5h5"/>',
    "prediction": '<path d="M4 19a9 9 0 1 1 16 0"/><path d="M12 15l4.5-5.5"/><circle cx="12" cy="15" r="1.2" fill="#000"/><path d="M4.5 19h15"/>',
    "doe": '<rect x="4" y="4" width="16" height="16" rx="1"/><circle cx="8.5" cy="8.5" r="1.1" fill="#000"/><circle cx="15.5" cy="8.5" r="1.1" fill="#000"/><circle cx="8.5" cy="15.5" r="1.1" fill="#000"/><circle cx="15.5" cy="15.5" r="1.1" fill="#000"/>',
    "flowchart": '<rect x="3" y="4" width="8" height="6" rx="1"/><path d="M15.5 13.5l4 4.5-4 4.5-4-4.5z" transform="translate(1.5 -3)"/><path d="M7 10v3h6"/>',
    "markov": '<circle cx="7" cy="8" r="3.2"/><circle cx="17" cy="16" r="3.2"/><path d="M10 6.5A8 8 0 0 1 18.5 11"/><path d="M18.9 7.8l-.4 3.4-3.2-.9"/>',
    "qcp": '<rect x="5" y="3" width="14" height="18" rx="2"/><path d="M8.5 6.5h7"/><circle cx="9" cy="11" r="0.6" fill="#000"/><circle cx="12" cy="11" r="0.6" fill="#000"/><circle cx="15" cy="11" r="0.6" fill="#000"/><circle cx="9" cy="14.5" r="0.6" fill="#000"/><circle cx="12" cy="14.5" r="0.6" fill="#000"/><circle cx="15" cy="14.5" r="0.6" fill="#000"/><circle cx="9" cy="18" r="0.6" fill="#000"/><circle cx="12" cy="18" r="0.6" fill="#000"/><circle cx="15" cy="18" r="0.6" fill="#000"/>',
    "rcm": '<circle cx="12" cy="12" r="3.2"/><path d="M12 2.8v3M12 18.2v3M2.8 12h3M18.2 12h3M5.5 5.5l2.1 2.1M16.4 16.4l2.1 2.1M18.5 5.5l-2.1 2.1M7.6 16.4l-2.1 2.1"/>',
    "fracas": '<path d="M6 21V4"/><path d="M6 5c4-2.2 8 2 12 0v8c-4 2-8-2.2-12 0"/>',
    # Actions
    "plus": '<path d="M12 5v14M5 12h14"/>',
    "save": '<path d="M12 3v10M8 9.5L12 13l4-3.5"/><path d="M4 16v3.5A1.5 1.5 0 0 0 5.5 21h13a1.5 1.5 0 0 0 1.5-1.5V16"/>',
    "open": '<path d="M12 13V3M8 6.5L12 3l4 3.5"/><path d="M4 16v3.5A1.5 1.5 0 0 0 5.5 21h13a1.5 1.5 0 0 0 1.5-1.5V16"/>',
    "trash": '<path d="M4 7h16M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2M6.5 7l1 13h9l1-13"/>',
    "copy": '<rect x="9" y="9" width="11" height="11" rx="1.5"/><path d="M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1"/>',
    "play": '<path d="M8 5.5v13l11-6.5z"/>',
    "export": '<path d="M14 4h-8v16h13v-8"/><path d="M13 4h7v7"/><path d="M20 4L11 13"/>',
    "link": '<circle cx="9" cy="14" r="5.5"/><circle cx="15" cy="10" r="5.5"/>',
    "star": '<path d="M12 3.5l2.5 5.2 5.7.7-4.2 3.9 1.1 5.6L12 16.2l-5.1 2.7L8 13.3 3.8 9.4l5.7-.7z"/>',
    "home": '<path d="M4 11l8-7 8 7"/><path d="M6 9.5V20h12V9.5"/>',
    "check": '<path d="M4.5 12.5l5 5L19.5 7"/>',
    "x": '<path d="M6 6l12 12M18 6L6 18"/>',
    "chart": '<path d="M4 3v17h17"/><path d="M6 15c3-1 5-3 7-5 1.6-1.6 3-2.6 5-3"/>',
    "table": '<rect x="3" y="5" width="18" height="14" rx="1"/><path d="M3 10h18M3 15h18M9.5 5v14M16 5v14"/>',
    "file": '<path d="M6 3h8l4 4v14H6z"/><path d="M14 3v4h4"/>',
    "dataset": '<ellipse cx="12" cy="5.5" rx="7" ry="2.8"/><path d="M5 5.5v13c0 1.6 3.1 2.9 7 2.9s7-1.3 7-2.9v-13"/><path d="M5 12c0 1.6 3.1 2.9 7 2.9s7-1.3 7-2.9"/>',
}

for name, body in ICONS.items():
    with open(os.path.join(OUT, name + ".svg"), "w") as f:
        f.write(HEAD + body + TAIL)

# Logo: rounded square outline in tint, reliability curve in light
# gray stepping down, solid tint dot at the curve start. Colors baked.
logo = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48">
<rect x="3" y="3" width="42" height="42" rx="10" fill="none" stroke="#0A84FF" stroke-width="2.6"/>
<path d="M11 13c8 1.5 14 5.5 19 12 2.4 3.1 4.2 5.6 7 7.5" fill="none" stroke="#F2F2F7" stroke-width="2.6" stroke-linecap="round"/>
<circle cx="11" cy="13" r="3" fill="#0A84FF"/>
</svg>'''
with open(os.path.join(OUT, "..", "logo.svg"), "w") as f:
    f.write(logo)

# Hero: wide flat panel artwork. Reliability curves (tint plus a
# fainter second curve), three outline blocks joined by lines in the
# lower right, a row of small dots along the bottom.
hero = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 340">
<path d="M30 30 C 300 45, 620 130, 900 235 C 1010 276, 1100 292, 1170 300" fill="none" stroke="#0A84FF" stroke-width="5" stroke-linecap="round"/>
<path d="M30 18 C 340 30, 700 95, 980 180 C 1060 205, 1120 218, 1170 224" fill="none" stroke="#5A5B60" stroke-width="3.5" stroke-linecap="round"/>
<g fill="none" stroke="#8E8E96" stroke-width="3">
<rect x="800" y="238" width="104" height="56" rx="8"/>
<rect x="936" y="238" width="104" height="56" rx="8"/>
<rect x="1072" y="238" width="104" height="56" rx="8"/>
<path d="M904 266h32M1040 266h32" stroke="#0A84FF"/>
</g>
<g fill="#6E6E76">
<circle cx="60" cy="316" r="5"/><circle cx="110" cy="316" r="5"/><circle cx="160" cy="316" r="5"/><circle cx="210" cy="316" r="5"/><circle cx="260" cy="316" r="5"/><circle cx="310" cy="316" r="5"/><circle cx="360" cy="316" r="5"/><circle cx="410" cy="316" r="5"/><circle cx="460" cy="316" r="5"/><circle cx="510" cy="316" r="5"/>
</g>
</svg>'''
with open(os.path.join(OUT, "..", "hero.svg"), "w") as f:
    f.write(hero)

print("wrote", len(ICONS), "icons + logo + hero to", OUT)
