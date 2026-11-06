# ReliaKit Design System

Original design work for ReliaKit, structured on Apple's Human
Interface Guidelines (HIG): clarity, deference, depth. No Apple
fonts, SF Symbols, ReliaSoft icons or artwork, stock photos, or
other third party art are used. The logo, hero graphic, and every
icon are original vector shapes, drawn as SVG assets
(gui/assets/icons/, gui/assets/logo.svg, gui/assets/hero.svg).

## Implementation

As shipped in v0.3.0, the interface is a web UI (HTML, CSS, and
vanilla JavaScript modules) hosted two ways from one codebase:

- Desktop: a Tauri 2 shell whose only job is the window, the app
  icon, and one `dispatch` command. All state and computation live
  in the reliakit-engine Rust crate; the front end sends JSON
  actions and renders the JSON state that comes back.
- Browser demo: the same front end files, with the same engine
  compiled to WebAssembly, served from site/demo/ on GitHub Pages.

An earlier v0.3.0 build used the Slint toolkit; it was superseded
mid-release by the user's direction to use whatever engine makes
the product feel modern and fast. The design tokens, icon set,
logo, hero artwork, and screen structure carried over unchanged;
only the rendering layer changed. The design tokens live in
gui/styles/main.css.

The layout target is a 1600 by 1000 window. The application opens
at that size so screenshots and daily use share the same
proportions. Standard form controls are native HTML elements
restyled to the tokens; there is no component library look.

## The three HIG principles

### 1. Clarity

- No body text under 12 px. Body is 14 px, captions are 12 px.
- Strong contrast on the dark theme: primary text #F2F2F7 on
  surfaces #17181A to #212326 is a contrast ratio above 12 to 1.
  Muted text #A7A7AD stays above 7 to 1 on the same surfaces.
  WCAG 2.2 AA spot-check (v0.3.2 gate, computed from the theme
  variables): body text 14.1 to 16.5 to 1, muted text 6.6 to
  7.4 to 1, status badge text 4.6 to 11.2 to 1, all pass the
  4.5 to 1 normal-text ratio. Three pairs measured below 4.5
  to 1 in the first pass and were fixed in v0.3.2: caption text
  (--faint) moved from #6E6E76 (3.1 to 3.5 to 1) to #8B8B93
  (4.7 to 5.3 to 1); text links and headline values use
  --tint-text #3B9DFF (5.6 to 1 on cards) instead of the brand
  tint #0A84FF (4.3 to 1 as text); primary buttons and the
  selected tree row now sit on --tint-pressed #0970D8, so
  white text measures 4.9 to 1 instead of 3.7 to 1. The brand
  tint itself is unchanged for fills, charts, focus rings, and
  the hero gradient, where it is not text.
- Headline results are large; supporting detail is body or caption
  in the muted color, so the eye lands on the result first.
- One icon per item kind, drawn from two or three plain shapes on
  an 18 px grid, recognizable at sidebar size.
- Numbers and labels say exactly what they are: "Reliability at
  t = 1000", not "Result". Empty states name the next action.

### 2. Deference

- Minimal chrome: one toolbar, one source list, one content area,
  one status line.
- Neutral surfaces everywhere. Color is spent sparingly, mostly
  the single tint accent #0A84FF, on the selection highlight, the
  primary action, links between items, and the first chart series.
- Plots and the RBD canvas sit directly on the content surface,
  not inside nested decorated cards. Only results get a card.
- Hover and selection are state changes on the same control:
  backgrounds and borders transition over 150 to 250 ms, selection
  fills with the tint, hover lifts to the raised neutral fill.
- Advanced options stay collapsed until opened (the Prediction
  screen's sections, the RBD diagram JSON). Progressive
  disclosure: the common path stays short.

### 3. Depth

- Layers are tonal, not shadowed, inside the app: base #131416,
  panels #17181A, raised cards #212326, hairline borders of white
  at low alpha. The start page hero is the one place with real
  depth: layered gradients and soft shadows, because it is the
  product's front door.
- Motion is functional: panel changes, selection, and hover
  transitions only, all 150 to 250 ms with an ease curve. Nothing
  animates on a loop.

## The start page and the polish benchmark

The stated benchmark for finish is Stripe's marketing site:
precise typography with a strong headline hierarchy, generous
spacing on an 8 point rhythm, subtle gradients and soft shadows
instead of flat boxes, rounded cards with hairline borders, and
restrained motion. The start page is built to that standard:

- Hero band: layered linear gradients from deep navy #0A1B33 to
  the tint blue family, with two radial-gradient glow circles
  softened by CSS blur. Web CSS does real gradients and blur, so
  no approximation was needed here; the egui and Slint builds
  could not do this and it is the main visual reason the web UI
  replaced them.
- Headline in Inter SemiBold at display size, one primary action,
  the ReliaKit mark, and a reliability curve visual drawn from the
  original hero artwork.
- Examples and datasets as cards on the 8 point grid with hairline
  borders, soft shadows, and a hover lift.

The same language carries to the landing page (site/index.html),
which is a single self-contained HTML file in the same palette.

## Tokens

- Tint: #0A84FF. Semantic: green #30D158, orange #FF9F0A, red
  #FF453A, purple #BF5AF2, teal #64D2FF, yellow #FFD60A.
- Chart series, in order: #0A84FF, #64D2FF, #BF5AF2, #30D158,
  #FF9F0A, #FF6482, #AC8E68, #A7A7AD.
- Radius: 10 px cards, 7 px controls.
- Type: Inter (bundled, SIL Open Font License 1.1) in Regular,
  Medium, SemiBold, Bold. Scale: display 40, title 22, headline
  17 SemiBold, body 14, caption 12, tiny 11.
- Spacing: 8 point rhythm (8, 16, 24, 32).

## Charts

Charts are inline SVG generated by the front end from series data
the engine returns: axes, tick labels, grid lines, and a legend
when a chart has more than one series. SVG keeps lines and text
crisp at any window size and prints cleanly from the browser demo.

## Icons and artwork

Thirty-four original SVG glyphs (one per item kind, plus shell
actions), the ReliaKit logo mark (a reliability curve falling
through an open square), and the hero artwork (reliability curves
over a row of diagram blocks). All were drawn for this project;
no third party icon pack or emoji is used anywhere in the UI.

## Navigation model and tour overlay (v0.3.1)

The v0.3.1 navigation model replaces the Add item dropdown. It is
part of the design system and uses the same tokens, cards, and
transitions as the rest of the interface:

- Visual picker. Adding an item opens a dialog of cards, one per
  kind, grouped under the four family headings (Life data analysis,
  System modeling, Risk and maintenance, Prediction and design).
  Each card carries the kind icon, name, and one-line description,
  with an optional name field above the groups. The start page
  reuses the same card shape for its Start an analysis section.
- Grouped project tree. The sidebar item list is grouped under the
  same family headings, with a count per family and a collapse
  control per group, so the picker, the tree, and the Tours list
  all speak in the same four families.
- Quick switcher. Ctrl+K (Cmd+K on macOS) opens a Jump to dialog: a
  filter field over a flat list of items, New entries for every
  kind, and the project actions. One highlighted row, arrow keys,
  Enter to run, Escape to close. It is a command surface, not a
  menu, and it works from any focus state.
- Item headers. Every item screen opens with the same header: kind
  icon, editable name, kind label, one-line description, and the
  Tour this screen and Remove actions. Screens differ below the
  header, never in it.
- Tour overlay layer. Guided tours render outside the app root, in
  a body-level layer the render cycle never rebuilds: a spotlight
  ring on the target control, a step card placed clear of it, and
  the Tours list as a wide dialog in the picker dialog shape. The
  overlay follows re-renders and resizes rather than freezing the
  screen, and it adds no new colors or type sizes.

## Layout, status visuals, and content security (v0.3.2)

### Layout changes (item M)

- Resizable panels. The project sidebar and the inspector are
  resizable by dragging their dividers (a 6 px sidebar handle and
  a 7 px inspector handle at its left edge, both highlighting with
  the tint on hover), and each can be collapsed entirely. The
  content area takes the freed space; no new chrome is added.
- RBD viewport. The RBD canvas keeps a view transform (pan and
  zoom from 0.2 to 2.5) in the front end. A small toolbar above
  the canvas carries Fit to view, zoom in, zoom out, the zoom
  percentage, and reset. Dragging empty canvas pans; Ctrl+scroll
  zooms at the pointer. A diagram whose block set is new is fitted
  automatically, so every block starts inside the visible canvas.
  Block positions themselves are unchanged by the view: the
  transform is a view state, not diagram data.
- Density. Results render as a compact summary grid of label and
  value cells instead of a stack of tall cards, with a Details
  disclosure under the grid that expands to a full table of every
  row. Long explanatory text moves into collapsed Help disclosures
  on the screens that had it inline, so the working surface stays
  dense and the text is one click away.
- Start page. The Quickstart panel becomes a compact collapsible
  card after its first dismissal. Recent projects and an unsaved
  work recovery prompt appear on the start page; both reuse the
  existing card and dialog shapes.

### Fit status and outdated badge visuals

- Outdated badge. One badge shape serves every surface: the
  sidebar item row, the headline card, and the results header line
  all show the same Outdated badge (class badge-outdated, orange
  family). An outdated headline card also takes an outdated class
  and prefixes its value with "Outdated:", so a stale number never
  reads as a fresh one. The results header line pairs the badge
  with one sentence saying the inputs changed and to run again.
- Fit status badges. One badge per status (class badge-fit-success,
  badge-fit-warn, badge-fit-failed): green for Fit OK, yellow for
  Fit OK with warnings and for Boundary or degenerate fit, red for
  Fit failed. The status note sits under the badge in body text and
  warnings list below it. No new type sizes or colors beyond the
  existing semantic set were introduced.
- Validation errors. A structured input error renders as a banner
  at the top of the content area naming the field, the reason, and
  the offending value, in the red semantic color. It is a banner,
  not a toast, so it stays until the next action replaces it.

### Content security policy

Shipped in v0.3.2. The Tauri configuration previously set the
content security policy to null (no policy). It now sets:

```
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'self'
```

The front end is hand-written modules and external stylesheets
loaded from the app itself, with no inline script, no remote
origins, and charts drawn as inline SVG markup rather than image
loads, so the policy holds without exceptions beyond the style
attribute allowance. Gate verification (Nov 6, 2026): the app
crate compiles with the policy in place, the desktop binary
passes its --headless self-check, and the desktop app was run
under Xvfb with this policy set; the start page rendered in
full (screenshot 16-desktop-csp.png in the v0.3.2 gate
evidence folder), so the policy ships as written.

## Platform notes

- The desktop window icon is set at runtime from the bundled
  logo PNG; the Windows executable icon and version metadata come
  from the Tauri bundle configuration (icons/icon.ico).
- The macOS download is a raw binary, not an .app bundle, so it
  has no bundle icon; it runs from Terminal. This is stated in the
  release notes and the user guide rather than disguised.
- On Linux the webview is the system WebKitGTK runtime
  (libwebkit2gtk-4.1); the app cannot bundle it. Windows and
  macOS use the webviews that ship with the OS.
