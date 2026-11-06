# ReliaKit Design System

Original design work for ReliaKit, structured on Apple's Human
Interface Guidelines (HIG). No Apple fonts, SF Symbols, ReliaSoft
icons or artwork, stock photos, or other third party art are used.
The logo, hero graphic, and every icon are original vector shapes,
drawn as SVG assets and rendered by the Slint UI toolkit
(gui/ui/icons/, gui/ui/logo.svg, gui/ui/hero.svg).

The layout target is a 1600 by 1000 window. The application opens at
that size so screenshots and daily use share the same proportions.
Standard widgets (combo boxes, line edits, check boxes, scroll
views) use the Cupertino dark widget style that ships with Slint;
every custom control, card, and panel follows the tokens in
gui/ui/theme.slint.

## The three HIG principles

Every major choice in this document and in the build maps to one of
the three HIG principles: Clarity, Deference, and Depth.

### 1. Clarity

Legible text at every size, precise and recognizable icons, subtle
decoration only, form follows function.

- No body text under 12 pt. Body is 14 pt, captions are 12 pt, and
  the smallest text in the app is the 12 pt caption style.
- Strong contrast on the dark theme: primary text `#F2F2F7` on
  surfaces `#17181A` to `#212326` is a contrast ratio above 12 to 1.
  Muted text `#A7A7AD` stays above 7 to 1 on the same surfaces.
- Headline results are large (screen title size or the strong status
  bar headline); supporting detail is set in body or caption and in
  the muted color, so the eye lands on the result first.
- One icon per item kind, each drawn from two or three plain shapes
  on an 18 px grid, recognizable at sidebar size. No decoration is
  used that does not label, group, or show data.
- Numbers and labels say exactly what they are: "Reliability at
  t=1000", not "Result". Empty states name the next action.

### 2. Deference

The interface recedes and the content leads.

- Minimal chrome: one toolbar, one source list, one content area,
  one status line. No decorative panels compete with plots,
  canvases, tables, or the RBD diagram.
- Neutral surfaces everywhere. Color is spent sparingly, mostly the
  single tint accent, on the selection highlight, the primary
  action, links between items, and the first chart series.
- Plots and the RBD canvas sit directly on the content surface, not
  inside nested decorated cards. Only results get a card, because a
  result is content the user may copy or act on.
- Hover and selection are state changes on the same control, not a
  swap to a decorated variant: backgrounds and borders cross-fade
  over 100 to 120 ms (Slint `animate` transitions), selection fills
  with the tint, hover lifts to the raised neutral fill.
- Advanced options stay collapsed (collapsing sections on the
  Prediction screen, the RBD diagram JSON, allocation and derating)
  until the user opens them. This is progressive disclosure: the
  common path is short, the full path is present.

### 3. Depth

Layers convey hierarchy.

- The sidebar (source list) is its own layer: a tonal step darker
  than the content, full height, visually distinct from the content
  layer beside it.
- The toolbar is a layer above the content: a tonal step lighter
  than the content area below it, separated by a 1 px separator,
  holding project actions on the left, the current item title in
  the middle, and version and primary project actions on the right.
- Result cards sit above the content surface: panel fill one tonal
  step up, 1 px border, 10 px radius. The dark theme uses no drop
  shadows; tonal steps and borders carry the layering. The RBD
  property editor reads as an inspector panel in the same
  treatment, in the right panel rather than a mode switch.
- Translucency and vibrancy: this build does not fake system
  vibrancy. Layers are flat with tonal separation, exactly the
  fallback the HIG allows when translucency is unavailable. This is
  stated here rather than imitated with translucent fills that
  would hurt text contrast.

## HIG patterns in the build

- Source list sidebar. The project tree is a macOS style source
  list: a PROJECT section with the project name, an ITEMS section
  with icon plus label rows grouped by nothing but the project
  itself, one Home row returning to the start page, and an ADD
  section with the kind picker, name field, and add control.
  Selection is a tint filled row with white text.
- Unified toolbar. Save project, Open project, the project name,
  and the current item title live in the top toolbar. The bottom
  status bar is kept from the earlier shell as the results line.
- Content-first single window. One window, a fixed 274 px source
  list sidebar, content takes the rest: inputs on the left, the
  plot, diagram, or table in the center, inspector on the right.
- Inspector, not modes. The RBD block property editor is a labeled
  inspector section shown when a block is selected. It edits in
  place and never replaces the canvas.
- Keyboard shortcuts. The v0.3.0 Slint build has no global
  shortcuts; project save and open are toolbar and sidebar buttons,
  and the RBD undo is a button on the RBD screen. (The retired egui
  build had Ctrl+S, Ctrl+O, Ctrl+Z, and Delete shortcuts; they did
  not carry over and are not claimed here.) Standard text field
  editing keys work in every field.
- Hit targets. Buttons and source list rows are at least 28 px tall
  through toolbar and button padding and a 28 px minimum interact
  size in the style.
- 8-point spacing rhythm. Spacing steps are 8, 16, 24, 32, with a
  4 px half step inside a control group: 4 inside groups, 8 between
  a label and its field, 16 panel and card padding, 24 between
  zones, 32 around the start page hero.
- Grouped settings in labeled sections. Every screen groups its
  inputs under plain section headings, in the order inputs, canvas
  or plot, results.

## Typography

Inter is bundled with the app (gui/ui/fonts, SIL Open Font License
1.1) in four weights: Regular, Medium, SemiBold, and Bold. Apple SF
fonts are licensed assets and are not bundled. The same Inter files
are registered with the chart renderer, so plots and interface text
match.

| HIG style | Size | Use in ReliaKit |
| --- | --- | --- |
| Large title | 32 pt | Start page title only |
| Title | 24 pt | Screen title at the top of each item view |
| Headline | 15 pt, semibold | Section headings, card titles |
| Body | 14 pt | Labels, field text, table text, buttons |
| Caption | 12 pt | Hints, notes, secondary detail |

Font note: the scale above is the Theme type scale in
gui/ui/theme.slint (fs-large, fs-title, fs-headline, fs-body,
fs-caption). Long result text wraps inside its card; the v0.2.0 bug
where a one-line fit summary overflowed its card does not recur,
because result cards are laid out from their full wrapped height.

## Color palette

Neutral dark surfaces, one tint accent, semantic colors in the
spirit of Apple's system colors. This replaces any earlier two
accent wording: there is one tint.

| Role | Hex | Use |
| --- | --- | --- |
| Content background | `#17181A` | Main content surface |
| Sidebar surface | `#131416` | Source list layer, one step darker |
| Toolbar surface | `#212326` | Toolbar layer, one step lighter |
| Card surface | `#212326` | Result cards and inspector |
| Border | `#3A3C40` | 1 px separators and card borders |
| Text primary | `#F2F2F7` | Body and headings |
| Text muted | `#A7A7AD` | Captions, hints |
| Tint accent | `#0A84FF` | Selection, primary action, item links, first chart series |
| System green role | `#30D158` | Pass, success |
| System red role | `#FF453A` | Fail, errors, destructive action |
| System orange role | `#FF9F0A` | Warn, caution |
| System yellow role | `#FFD60A` | Bounds, attention detail |
| Chart series 2 | `#64D2FF` | Second plotted series |
| Chart series 3 | `#BF5AF2` | Third plotted series |
| Chart series 4 | `#FF6482` | Fourth plotted series |
| Chart series 5 | `#30D158` | Fifth plotted series |

Series colors beyond the first are used only inside plots, where
several data series must be told apart; interface chrome never uses
them. Charts are drawn with the plotters crate into an image buffer
at 1280 by 600 and shown in the center panel; they keep labeled
axes, and a legend names each series when more than one is plotted.

## Corners, borders, states

- Radius 10 px on cards, the inspector, and the hero; 7 px on
  buttons and fields.
- Cards: card surface fill, 1 px border, no drop shadow, 16 px
  inner margin.
- Hover: raised neutral fill, cross-faded over 120 ms.
- Selected: tint fill row in the source list; a 2.5 px tint stroke
  on the selected RBD block.
- Disabled: actions that need a fit first are replaced by an empty
  state sentence instead of a disabled control.
- Empty: a plain sentence naming the next step, for example "Run a
  fit first, then the send actions appear here."

## Layout principles

1. One primary action per screen (Clarity). Each item view has one
   main run or compute action; the rest are plain buttons in their
   own grouped sections.
2. Three zones (Deference). Inputs, then canvas or plot, then
   results, in reading order, so chrome never interrupts content.
3. Alignment grid (Clarity). One left edge for labels and fields;
   rows share baselines; the sidebar width is stable.
4. Whitespace (Deference). 24 px between zones on the 8 point
   rhythm; nothing decorative fills the gaps.
5. Results as cards with a large headline (Clarity, Depth). The
   headline value also appears in the status bar.
6. Empty states say what to do next (Clarity).

## Icons

Original icons, drawn to HIG clarity standards: one recognizable
idea per icon, plain shapes, consistent 1.6 px stroke on an 18 px
grid. Each icon is an original SVG file in gui/ui/icons, tinted to
the current text color so they sit quietly in the source list
(Deference). No icon fonts, packs, SF Symbols, or emoji.

Item icons, in the source list next to each item name:

- Life Data: a falling reliability curve.
- Limited Data: three dots with a bracket, for small groups.
- MCMC: three linked circles, for chains.
- Degradation: a falling slope crossing a dashed threshold.
- ALT: a triangle with a bolt, for accelerated stress.
- Growth: a rising curve.
- Competing Risks: one line splitting into two.
- RBD: two blocks joined by a line.
- Fault Tree: a root node splitting to two leaf nodes.
- FMEA: a small table grid.
- Warranty: a curved return arrow.
- Prediction: a gauge arc with a needle.
- DOE: a square of four design points.
- Flowchart: a rectangle feeding a diamond.
- Markov: two circles joined by an arc, for state transitions.
- QCP: a calculator rectangle with key dots.
- RCM: a circle with spokes, for a maintained asset.
- FRACAS: a flag on a pole, for a reported incident.

Action icons, drawn beside their button labels where the layout has
room (project panel, toolbar, start page):

- Add: a plus.
- Save: a downward arrow into a tray.
- Open: an upward arrow out of a tray.
- Delete: an X in a square.
- Duplicate: two overlapping squares.
- Run: a right pointing triangle.
- Export: an arrow leaving a square.
- Link or assign: two overlapping circles.
- Use (wizard): a star. Examples on the start page use the same star.

The wizard lives inside the Life Data view and uses the Life Data
icon in the tree. Examples are start page content, not an item kind.

## Logo and hero

Logo mark: a rounded square outline in the tint accent holding a
reliability curve in neutral light gray that steps down from the
top left, with a solid tint dot at the curve start. It is the
original SVG asset gui/ui/logo.svg, 28 px in the toolbar and 56 px
on the start page.

Hero graphic: a wide flat panel on the start page, the original SVG
asset gui/ui/hero.svg (Deference: it is the one decorative element,
kept quiet). A large reliability curve in the tint accent, a fainter
second curve in muted gray, three outline blocks joined by lines in
the lower right as a block diagram motif, and a row of small dots
along the bottom for test units.

## Screens restyled

All of them, through the shared toolbar, source list, status bar,
text styles, section headings, spacing, result cards, and palette:
the start page and every item view (Life Data, Limited Data, MCMC,
Degradation, ALT, Growth, Competing Risks, RBD, Fault Tree, FMEA,
Warranty, Prediction, DOE, Flowchart, Markov, QCP, RCM, FRACAS).
The RBD canvas uses the neutral content surface and palette strokes
instead of its earlier dark green and brown block fills.
