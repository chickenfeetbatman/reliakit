# Changelog

## v0.3.1

A fix and navigation update. The engine, the project file format
(reliakit-project-2), and all 18 item kinds are unchanged.

- Fixed RBD dragging. In v0.3.0, blocks could not be dragged to new
  positions. Now the whole block drags smoothly, and the new position
  is saved in the project file and restored when the project is
  reopened.
- Fixed RBD drawing (correctness). Diagrams now draw the topology
  that is actually evaluated. Parallel and k-out-of-n groups draw as
  branches between split and merge rails, standby groups show their
  standby unit, and nested groups are framed and labeled, so the
  picture matches the model. In v0.3.0 the bundled Example 3 drew
  its parallel controllers in series.
- Visual picker. Add item now opens a picker with a card for each of
  the 18 analysis kinds, grouped by family (Life data analysis,
  System modeling, Risk and maintenance, Prediction and design),
  each card with an icon and a one-line description. An optional name
  field sits at the top of the picker.
- Grouped project tree. The item list in the project panel is grouped
  under the same four family headings, with a count per family. Each
  group can be collapsed and expanded.
- Quick switcher. Ctrl+K (Cmd+K on macOS) opens a Jump to box that
  lists the project items, a New entry for each of the 18 kinds,
  Add item, Save project, Open project, Start page, and Tours. Type
  to filter, use the arrow keys and Enter, or click an entry.
- Item screen header. Every item screen now has the same header: an
  editable item name, the kind label, a one-line description, a
  Tour this screen button, and a Remove button.
- Start page. A Start an analysis section adds cards for Life Data,
  RBD, FMEA, Prediction, Markov, and FRACAS that create that kind of
  item in one click.
- Guided tours. A Tours button in the toolbar opens a list of tours
  for all 18 kinds, grouped by family with a step count per tour.
  Each tour sets up its own new item named Tour: plus the kind name,
  loads an example or starts fresh, and walks through one analysis
  step by step with a highlighted target and Back, Next, and Finish
  controls. Your existing items are not changed. Every item screen
  also has a Tour this screen button that starts the tour for that
  kind.
- First project wizard. The start page has a New here? Build your
  first project card with three goal paths: Analyze failure data
  (starts the Life Data tour), Model a system (starts the RBD tour),
  and Track failures and maintenance (starts the FRACAS tour).
- Browser demo updated with all of the above.

## v0.3.0

ReliaKit has a new interface, and it now runs in the browser as well as
on the desktop.

- New web interface. The desktop app is now built with Tauri and a web
  front end written for ReliaKit: a toolbar with New, Open, Save, and
  About, a project source list on the left, the item screen in the
  center, and a status bar with the headline result. The design follows
  the ReliaKit design system in docs/DESIGN.md, with the dark surfaces
  and single blue tint, an 8 point spacing rhythm, hairline borders,
  cards that lift on hover, and short ease transitions on hover,
  selection, and panel changes.
- Start page. A gradient hero heads the start page, followed by the
  project actions, a dismissible Quickstart panel that explains the
  project flow, the eight example cards, and the bundled dataset cards.
  The About view, open from the toolbar or the start page, carries the
  version, the repository link, and the license attributions.
- Live browser demo. The same interface runs on GitHub Pages at
  https://chickenfeetbatman.github.io/reliakit/demo/, with the same
  engine compiled to WebAssembly. In the demo, project Save downloads
  reliakit-project.json, project Open and CSV import use a file picker,
  and controls that need a local file path on the desktop show a short
  note and a working picker or download instead.
- All 18 item kinds carried over with their full control sets: Life
  Data with the distribution wizard, Limited Data, MCMC, Degradation,
  ALT, Growth, Competing Risks, RBD, Fault Tree, FMEA with DVP&R,
  Warranty, Prediction with the MIL-HDBK-217F library and the
  clean-room SR-332 and 62380-style methods, DOE, Flowchart, Markov,
  QCP, RCM, and FRACAS. The six data flows between items work as
  before, and project files keep the reliakit-project-2 format, so
  v0.2.0 projects open unchanged.
- RBD canvas. Place blocks from the palette with a click, drag blocks
  to move them, and edit the selected block in the inspector, with
  connector lines drawn from the diagram. Availability and importance
  measures stay on the same screen.
- Charts are drawn in the interface as labeled vector charts with axes
  and legends, in the same series colors as before.
- Keyboard shortcuts: Ctrl+S saves the project, Ctrl+O opens a project,
  and Delete removes the selected RBD block or the selected FRACAS,
  RCM, or FMEA row when you are not typing.
- Errors show in the status bar and in a visible banner, and compute
  actions show a working indicator on the button while they run.
- A product landing page now fronts the project, with feature sections,
  screenshots, and download links for all six release files.
- A full user guide is included at docs/USER_GUIDE.md, covering every
  item kind, the data flows, the Prediction standards, file formats,
  shortcuts, and platform notes.
- Interface history, in one line: v0.3.0 replaced the GUI toolkit; the
  earlier desktop interfaces are retired.

## v0.2.0

One project workspace with 18 item kinds and data flows between items, seven new distributions, Markov, RCM, FRACAS, FMECA, the distribution wizard, QCP, importance measures, DOE, RGA, and ALTA expansions, the MIL-HDBK-217F Notice 2 library built from the published handbook, clean-room SR-332 and 62380-style methods, and 16 sourced bundled datasets.

## v0.1.0

First Rust release: the reliability library, command line tool, and desktop app, with life data fitting, limited data methods, MCMC, system RBD with standby, and the worked examples.
