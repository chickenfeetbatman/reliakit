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
