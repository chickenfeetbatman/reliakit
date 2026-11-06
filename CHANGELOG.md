# Changelog

## v0.3.2

A correctness and validation release. The project file format
(reliakit-project-2) and all 18 item kinds are unchanged.

Fixed defects, in plain terms:

- Fractional times were truncated. A time such as 0.1 or 100.25
  could be stored as a whole number after CSV import or an example
  load. Stored values now keep their full precision everywhere,
  and a save and reopen round trip leaves them unchanged.
- Invalid input was silently dropped or replaced. A bad token in
  a data field, an invalid mission time, or an overflowing number
  could be ignored while a calculation ran on partial data. Bad
  input now stops the run with an error that names the field, the
  value, and the reason, and earlier results are kept and marked
  outdated rather than overwritten.
- Fits could report success on data that has no usable fit, such
  as all zero times or all identical times. Fits now carry one of
  four statuses (success, success with warnings, boundary or
  degenerate, failed), shown as a badge with the reason, and a
  failed fit has no headline result.
- The Weibull shape had a hidden cap. Tightly clustered times
  (100 to 104) hit an internal bracket and returned a shape near
  20 instead of the correct value near 81. The bracket now expands
  to fit the data, and data whose shape is truly unbounded is
  reported as a boundary case instead of a clean fit.
- Fault trees mishandled repeated events. A OR A returned 0.19
  instead of 0.1, because the two copies of A were treated as
  independent. Repeated basic events now share one identity in
  every gate, probabilities outside 0 to 1 are errors that name
  the event, and large trees with repeats error clearly instead
  of approximating.
- Warranty forecasts counted returns from the future. Returns
  recorded after the current period entered the fit. They are now
  excluded, the exclusion count and cutoff are stated in the
  result notes, the age convention (a return in the sale period
  is a failure at age 1) is applied consistently, and a cohort
  with more returns than sales is an error naming the cohort.
- The shifted exponential fit used the wrong starting point when
  a suspension came before the first failure. The location now
  sits at the smallest failure time, with the corrected scale.

New validation:

- A validation fixture record (validation/fixtures.md) and a new
  test file (tests/validation_tests.rs) check distribution
  identities by numerical integration, recover known parameters
  from constructed samples, recompute Fisher standard errors by a
  second path, check RBD and standby closed forms, and compare
  Monte Carlo availability with the CTMC value within 3 standard
  errors. A committed browser test suite (browser-tests/) covers
  the import round trip, the outdated badge, fit to RBD sends,
  save and reopen, a keyboard-only workflow, all 18 tours, and
  the RBD viewport. CI now also checks formatting, clippy, and
  the CLI selftest.

Interface changes (items M and N, and K):

- Panels resize and collapse; the RBD canvas gains a viewport
  toolbar (Fit to view, zoom, pan) and fits new diagrams
  automatically; results show as a compact grid with an
  expandable Details table.
- Items can be duplicated from the panel or the screen header;
  the project tree has a Filter items search; send actions say
  they copy a snapshot; the Life Data probability plot is titled
  "Empirical vs fitted probability".
- Results carry an Outdated badge in the panel, on the headline
  card, and above the results whenever inputs change after a run,
  and sending outdated results asks for confirmation first.
- RBD availability takes a Simulations count and reports a Monte
  Carlo standard error.
- The desktop app sets a restrictive content security policy and
  gains a working headless smoke check (reliakit-app --headless
  fits Example 1 and verifies its headline).

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
