# ReliaKit User Guide

ReliaKit v0.3.1 is a reliability engineering suite. It runs as a desktop
app on Windows, macOS, and Linux, and as a live demo in a web browser at
https://chickenfeetbatman.github.io/reliakit/demo/. Both run the same
engine and the same interface. This guide covers the project workspace,
every item kind, the data flows between items, the Prediction standards,
the file formats, the keyboard shortcuts, and platform notes.

## Getting started: projects and items

A project holds items. Each item is one analysis: a life data fit, a
system diagram, an FMEA worksheet, and so on. Each item keeps its own
inputs and results, so you can keep every analysis for one product in a
single project file and move between them without losing work.

- The start page is home. It shows the first project wizard card (see
  the Tours section), the Quickstart panel, three project actions (New
  project, Open project, and Save project for the current project), a
  Start an analysis section (see below), eight example cards, and the
  bundled dataset cards.
- The project panel on the left is the source list. It shows the project
  name, a Start page row, the item list grouped by family, an Add item
  button, and the project file controls.
- To add an item, choose Add item in the project panel (or on the
  start page, see Start an analysis below). The visual picker opens:
  a card for each of the 18 kinds, grouped by family (Life data
  analysis, System modeling, Risk and maintenance, Prediction and
  design), each card with an icon, the kind name, and a one-line
  description. Type a name in the Name (optional) field at the top if
  you want one, then click a card. The new item opens in the content
  area. Choose Cancel or click outside the picker to close it without
  adding anything.
- The item list is a grouped, collapsible project tree. Items sit
  under their family heading with a count for the family. Click a
  family heading to collapse or expand that group. Click an item row
  to open it.
- Start an analysis cards. The start page has a Start an analysis
  section with cards for Life Data, RBD, FMEA, Prediction, Markov,
  and FRACAS. Each card shows the kind name and its one-line
  description. Click a card to create an item of that kind and open
  it. For the other 12 kinds, use Add item and the picker.
- Every item screen has the same header at the top: the kind icon, an
  editable name field, the kind label, and a one-line description of
  the kind, plus two buttons on the right, Tour this screen and
  Remove. To rename an item, edit the name in the header and leave
  the field (the rename applies when the field loses focus). To
  remove the item, choose Remove in the header.
- To jump between items without the mouse, use the quick switcher.
  Press Ctrl+K (Cmd+K on macOS), or choose Jump to in the toolbar.
  The switcher lists your project items, a New entry for each of the
  18 kinds, Add item (which opens the picker), Save project, Open
  project, Start page, and Tours. Type to filter the list, move with
  the Up and Down arrow keys, press Enter to run the highlighted
  entry, click an entry to run it, and press Escape or click outside
  to close it. Choosing a New entry creates an item of that kind;
  choosing an item entry opens that item.
- Examples load as new items in the current project. Example 1 is a
  bearing life data fit. Example 2 is limited data lots. Example 3 is an
  RBD with cold standby pumps and parallel controllers. Example 4 is an
  FMEA sample. Example 5 is an ALT dataset. Example 6 is a growth
  dataset. Example 7 is degradation units. Example 8 is the Markov
  two-unit preset, whose MTTF from Both up is 650 hours.
- Bundled datasets also load as new items. Life files go to Life Data,
  ALT files to ALT, degradation files to Degradation, growth files to
  Growth, and the Nevada warranty file to Warranty. Each file and its
  source are recorded in datasets/README.md in the repository.
- Most item screens share one layout: inputs on the left, the chart,
  table, or canvas in the center, and an inspector with result cards on
  the right. The headline result also appears in the status bar at the
  bottom of the window.
- The Quickstart panel on the start page sums up the flow in five
  steps: projects hold items, add an item from the visual picker, load
  data or an example, run, and send results between items. Dismiss it
  when you are done with it. You can reopen it from the About view,
  which also carries the version, the repository link, and the license
  attributions.
- The toolbar holds New, Open, and Save for the project, a Jump to
  button (the quick switcher), a Tours button (see the Tours section),
  the current project title, the version, and About.
- Errors are never silent. The engine reports problems in plain language
  in the status bar, and error notes also appear in a visible banner
  that stays until you dismiss it or it times out.
- Actions that compute, such as fits, MCMC, Markov, and availability,
  show a working indicator on the button while the calculation runs.

Quickstart, in short:

1. Start a project, or open an existing project file.
2. Add an item of the kind you need.
3. Type or paste data, import a CSV, or load an example or dataset.
4. Run the analysis with the primary button on the screen.
5. Read the result cards, then send the result to another item if it
   feeds a second analysis.

## Tours and the first project wizard

Guided tours walk through one analysis at a time, step by step. There
is a tour for each of the 18 kinds. Each step highlights the control
it talks about and shows a short card with the step title and text,
plus Back, Next, and a close control. The last step ends with Finish.

- Tours never touch your work. Starting a tour creates its own new
  item named Tour: plus the kind name (for example, Tour: Life Data),
  loads the bundled example or dataset for that kind where there is
  one, or starts a fresh item where there is not, and runs inside
  that item. Close or remove the tour item when you are done with it.
- Toolbar Tours list. Choose Tours in the toolbar to open the Guided
  tours list. Tours are grouped by family, each row with the kind
  icon, name, one-line description, and step count, and a Start
  button.
- Tour this screen. Every item screen header has a Tour this screen
  button. It starts the tour for the kind of the item you are looking
  at, in a new tour item as described above.
- First project wizard. The start page card titled New here? Build
  your first project has three goal paths. Each path sets up a
  starter item and starts its tour: Analyze failure data starts the
  Life Data tour, Model a system starts the RBD tour, and Track
  failures and maintenance starts the FRACAS tour. Your existing
  items are not changed.

## The item kinds

### 1. Life Data

Fit a life distribution to failure and suspension times. Type times
comma separated; a suffix of c or + marks a censored (suspended) unit.
Pick one of 14 distributions (weibull2, weibull3, lognormal, normal,
exponential, gamma, gumbel, loglogistic, logistic, gompertz, gengamma,
weibull1, exponential2, mixed_weibull), set a mission time t, and choose
Run fit. Fitting is by maximum likelihood, or by median rank regression
when Rank regression is checked. Results include the fit summary,
reliability at t with bounds, B10, and mean life. The center shows the
reliability curve, or the probability plot when Show probability plot
is checked.

The distribution wizard lives on this screen. Choose Rank distributions
to rank the candidate distributions on the current data by log
likelihood, AIC, and BIC, with the Anderson-Darling statistic where it
is available. Choose Use on a ranked row to make that distribution the
Life Data distribution. See the data flows section for the three send
actions that appear after a fit.

CSV import on this screen accepts the bundled datasets and, on the
desktop, a CSV file path. In the browser demo, Load CSV file opens a
file picker instead of asking for a path.

### 2. Limited Data

Hierarchical shrinkage for small lots, with bias correction and
conformal bounds. Enter one lot per line, with times comma separated
and c marking a censored unit, then choose Run hierarchical fit. Use
this when each lot has too few failures to fit on its own and the lots
should borrow strength from each other.

### 3. MCMC

Bayesian fitting by adaptive Metropolis sampling. Enter times (c marks
a censored unit), pick the weibull2 or lognormal model, and set draws,
warmup, chains, seed, and a mission time t, then choose Run MCMC. The
posterior summary includes parameter summaries, the posterior
reliability at t as the headline, and split R-hat and effective sample
size diagnostics. Sampling uses the seeded library sampler, so a run
with the same seed repeats exactly.

### 4. Degradation

Path fits per unit, pseudo-failure times at a threshold, then a life
fit on those times. Enter one unit per line in the form times, then a
pipe character, then measurements (for example: 100,200,300 |
98.1,96.4,94.0). Set the failure threshold, pick a linear, exponential,
or power path model, pick the life distribution for the pseudo-failure
times, and set Increasing failure direction if failure is upward
crossing rather than downward. Choose Run degradation analysis. The
center chart shows the fitted paths.

### 5. ALT (Accelerated Life Test)

Least squares and censored maximum likelihood fits, extrapolated to use
stress. Enter one unit per line as stress,time, or stress,stress2,time
for the Eyring model, with an optional trailing failed flag of 1 or 0.
Pick the arrhenius, ipl (inverse power law), or eyring model, set the
use stress and a mission time t, and choose Run ALT fit. The center
chart shows reliability at use stress. The library also supports
two-stress loglinear fits and step-stress profiles through the command
line; the item screen covers the three named models.

### 6. Growth (Reliability Growth)

Crow-AMSAA and Duane fits on cumulative failure times, plus Gompertz
and Lloyd-Lipow results from the same library code. Enter failure times
comma separated and choose Run growth fit. The center chart shows the
growth curve and fitted cumulative failures.

### 7. Competing Risks

Cause-specific fits and cumulative incidence per failure mode. Enter
one unit per line as time,mode; a blank mode means the unit was
censored. Pick a distribution, set a mission time t, and choose Run
competing risks fit. A failure with an unknown mode is treated as its
own unknown mode category. The risks are treated as independent.

### 8. RBD (System RBD)

A block diagram that evaluates with the system reliability engine,
including standby groups.

- Palette: Single block, Standby group, Parallel group, and k-out-of-n
  group. Pick a palette item to arm it, then click the canvas to place
  it. Click a placed block to select it. To move a block, press on
  any part of the block (the whole block is the drag handle, name
  included), move the pointer, and release. The block follows the
  pointer while you drag. A short press without moving is still a
  click and only selects the block. Block positions save with the
  project file and are restored when the project is reopened.
- Evaluation: set the mission time t, or set a topology (series,
  parallel, k-of-n, or series with standby group) with its k value and
  choose Apply topology. The headline card shows system reliability at
  t and updates as the diagram changes.
- Availability: set the availability horizon and choose Run
  availability (40 sims) for the Monte Carlo availability summary.
- Reliability importance: choose Compute importance measures for
  Birnbaum, Fussell-Vesely, and criticality values per block.
- Block inspector: when a block is selected, edit its name,
  distribution, parameters (written as beta=1.5, eta=5000), MTBF, and
  MTTR. Standby blocks also show units, dormancy, and switch
  reliability. Choose Apply properties to save the edit. Duplicate,
  Delete, and Undo act on the selected block and the diagram history.
- Add block form: the same properties can be set before placing, with a
  mode of active, hot standby, cold standby, or warm, and a dormancy
  value for warm standby. Choose Add block to append the block.
- Diagram JSON: the diagram in the library RBD JSON schema. Leaf nodes
  may carry an additive pos field with canvas coordinates. Load from
  JSON text applies the text in the box. On the desktop, Save and Load
  write and read the file path shown. In the browser demo, Save
  downloads the JSON and Load opens a file picker.

### 9. Fault Tree

Top event probability and minimal cut sets from a gate tree. Paste the
tree as FtNode JSON in the serde format (Basic, And, Or, and voting
gates), then choose Evaluate. Cut sets are exact for small trees; there
is no binary decision diagram engine, so very large trees can be slow.

### 10. FMEA

A Design, Process, or System worksheet with RPN and FMECA criticality.
The worksheet type is stored with the worksheet.

- Add row: fill in item, failure mode, severity, occurrence, and
  detection, plus the FMECA values (alpha, beta, lambda per hour, and
  mission hours) when you want criticality, then choose Add row. The
  worksheet shows item, failure mode, Sev, Occ, Det, RPN, revised RPN,
  and criticality per row.
- Each row has To RCM and Delete buttons. See the data flows section.
- DVP&R entry: fill in id, failure mode reference, test, sample size,
  and acceptance criteria, then choose Add DVP&R entry. Entries list
  below the worksheet.
- Worksheet CSV: Export worksheet to CSV text writes the full worksheet
  (a first line with the FMEA type, then the rows CSV) into the CSV text
  box. Export rows only writes just the rows. Import CSV reads the CSV
  text box back into the worksheet. Clear rows empties the worksheet.
- File exports: on the desktop, Export control plan CSV and Export
  DVP&R CSV write to the export file path. In the browser demo, exports
  download as files instead.

### 11. Warranty

Nevada table aggregation and a Weibull returns forecast. Enter sales
per period comma separated, and returns one per line as sale period,
return period, both 0-based. Set the current period and the horizon in
periods, then choose Run warranty analysis. The center shows the Nevada
table (rows are sale periods, columns are age in periods) and the
forecast results.

### 12. Prediction

Parts-count prediction, MIL-HDBK-217F part stress, reliability
allocation, derating, and clean-room method shapes. The screen has four
zones.

- Life model: a fitted distribution sent from a Life Data item, or set
  by hand with a distribution and parameters. Set a mission time and
  choose Compute R(t).
- Standard or source: pick one of three sources. Generic user data runs
  the parts-count form: one part per line as name, base rate per
  million hours, quantity, stress factor, quality factor, then Run
  prediction at a mission time. Local library file loads a component
  library CSV (same columns; comment lines starting with # are
  skipped) and lists its parts with system totals. MIL-HDBK-217F
  library runs the part stress form described in the standards section
  below.
- Reliability allocation: set a system target R, the number of
  subsystems, and weights for the weighted split, then choose Allocate
  for equal and weighted per-subsystem targets.
- Derating check: enter a part name, operating stress, rated stress,
  and a maximum allowed ratio, then choose Check derating. The result
  says whether the part passes at that limit. Built-in derating
  standards tables are not bundled; the limit is yours to set.
- Clean-room methods and the mission profile are covered in the
  standards section below.

### 13. DOE (Design of Experiments)

Two-level factorial analysis. Enter the number of factors k and the
2^k responses y in Yates order, then choose Analyze for effect
estimates, sums of squares, and percent contribution. The library also
builds fractional factorial, Plackett-Burman, and Taguchi designs and
fits response surface (central composite and Box-Behnken) models; this
screen runs the full 2^k analysis.

### 14. Flowchart

Process network simulation in the RENO style. Paste a process network
as JSON (start, task, decision, loop or counter, and end blocks), set
the number of simulations and a seed, and choose Run simulation for the
run time and cost distributions. Flows are JSON defined; there is no
graphical flow editor.

### 15. Markov

Continuous time Markov models with named states. The screen opens with
the two-unit repairable preset available on one click: Load two-unit
preset gives states Both up, One down, and System down, and an MTTF of
650 hours from Both up.

- States: type a new state name and choose Add state, or Remove on a
  state row to delete it and its transitions.
- Transitions: the center table lists from state, to state, and rate
  per hour. Click a transition to select it, edit it in the Transition
  inspector, and choose Apply edits, Remove transition, or Add
  transition.
- Compute: pick the start state and the absorbing state used for MTTF,
  set the transient time, and choose Compute Markov results for steady
  state probabilities, transient probabilities (also charted), and mean
  time to absorption.

### 16. QCP (Quick Calculation Pad)

Reliability quantities for one distribution at one mission time. Pick a
distribution; its parameter fields appear under Parameters and update
when the distribution changes. Set the mission time t and the Bx
fraction (0.10 means B10 life), then choose Calculate for reliability
(the headline), unreliability, hazard, mean life, pdf, and Bx life.

### 17. RCM

A reliability centered maintenance worksheet. Rows carry an asset, a
function, a failure mode, a consequence class (Hidden, Safety,
Environmental, Operational, NonOperational), three decision inputs
(condition monitorable, age related, restorable), and the FMECA values
(rate per hour, alpha, beta, mission hours) used for criticality.

- Choose Add row to start a blank row, click a row in the worksheet to
  select it, and edit it in the Row inspector. The selected task and
  the criticality Cm update from the row inputs; choose Refresh task
  and criticality after edits. Criticality needs a failure rate, mode
  ratio, effect probability, and mission hours.
- Choose Remove row to delete the selected row.
- Rows can also arrive from an FMEA worksheet. See the data flows
  section.

### 18. FRACAS

A failure reporting and corrective action incident log. Each incident
has an id, part, failure mode, hours, date, status (Open, Analysis,
CorrectiveAction, Closed), and a corrective action.

- The Summary cards show incident counts and MTBF against the total
  operating hours you enter.
- Choose Add incident to start a blank incident, click an incident in
  the table to select it, and edit it in the Incident inspector. Choose
  Apply edits to refresh the summary, or Remove incident to delete it.
- CSV import and export: on the desktop, Export CSV and Import CSV use
  the CSV file path. In the browser demo, Export CSV downloads the log
  and Import CSV opens a file picker.
- Send incidents to Life Data is covered in the data flows section.
  FRACAS here is a local log; there is no hosted portal or multi-user
  workflow.

## Data flows between items

Six actions move results from one item to another, so chained analyses
do not need retyping. Targets are found or created for you: a flow that
needs a QCP, Prediction, or RCM item creates one named QCP 1,
Prediction 1, or RCM 1 when the project has none.

1. Assign fit to RBD block (Life Data to RBD). Run a fit, then in the
   Send this fit panel pick the target RBD item and the block, and
   choose Assign fit to RBD block. The block takes the fitted
   distribution and parameters, and the RBD reliability recomputes when
   you open that item.
2. Send fit to QCP (Life Data to QCP). Run a fit, then choose Send fit
   to QCP. The QCP item takes the fitted distribution, parameters, and
   mission time. Open it and choose Calculate.
3. Send fit to Prediction (Life Data to Prediction). Run a fit, then
   choose Send fit to Prediction. The Prediction item life model takes
   the fitted distribution, parameters, and mission time, ready for
   Compute R(t).
4. FMEA row to RCM row (FMEA to RCM). Choose To RCM on a worksheet row,
   or type a 1-based row number in the Send a row to RCM section and
   choose Create RCM row. A new RCM row is created with the FMEA item
   as the asset, its function, and the failure mode, with consequence
   set to Operational for you to review in the RCM inspector.
5. FRACAS incidents to Life Data (FRACAS to Life Data). Choose Export
   to Life Data item. A new Life Data item is created with each
   incident hour value as a failure time. All incidents count as
   failures, because the FRACAS log has no censoring notion.
6. Wizard Use (wizard to Life Data). On the Life Data screen, after
   Rank distributions, choose Use on a ranked row. That distribution
   becomes the Life Data distribution, ready for Run fit. It stays in
   the same item; no new item is created.

## Prediction standards

The Prediction screen covers three standards families. What is built in
and what you must supply differs by standard, by design.

### MIL-HDBK-217F built-in library

MIL-HDBK-217F Notice 2 is a public US government document, so its part
stress library is built into ReliaKit. The library was transcribed from
the published handbook PDFs, with the source table recorded for each
category. It covers 18 part categories: capacitors, circuit breakers,
coils, connections, connector sockets, connectors, elapsed time
meters, electronic filters, fuses, interconnection assemblies with
PTHs, miscellaneous parts, quartz crystals, relays (mechanical and
solid state), resistors, switches, synchros and resolvers, and
transformers. Handbook sections 5, 6, 7, 8, 18, and 20 are not built,
with the reasons logged in FEATURE_RESEARCH.md in the repository. The
NSWC handbook library is not included.

To run a 217F calculation: set Standard or source to MIL-HDBK-217F
library, pick a category file, and choose Load category. Most
categories then show their factor inputs in the center panel as named
pickers and numeric fields that follow the handbook tables. The
miscellaneous parts category holds fixed rates instead: pick a part and
an environment (GB, GF, GM, NS, NU, AIC, CL), and the rate shown is the
fixed rate times the environment factor. Choose Compute part failure
rate for the result. The handbook capacitor worked example is covered
by a test: the library gives 0.04898 against the printed 0.049 failures
per million hours.

### SR-332 and 62380-style methods with user-supplied data

Telcordia SR-332 and IEC 62380 are paid standards. Their data tables
are licensed documents, so no table data from either standard is built
in, and nothing in ReliaKit reproduces them. What ships is the method
shapes, implemented clean-room from public secondary sources. Every
rate and factor is a value you supply from your own licensed copy.

The Clean-room methods section runs, on your numbers:

- Method I style device calculation: a generic rate with quality,
  stress, temperature (Arrhenius, with an activation energy in eV; 0.35
  eV is the vendor-cited default), and environment factors, plus a
  quantity and device rate roll-up.
- Method II style adjustment: an observed rate moved between observed
  and target temperature and stress factors.
- Method III style combination: a weighted combination of a predicted
  rate and an observed rate.
- Chi-square confidence bounds on a rate from a failure count and an
  exposure in hours, at your confidence level.

Choose Run clean-room calculations to compute all of them at once.

The 62380-style mission profile section is a mission profile framework
with your base rate and factors: set a base rate, activation energy,
reference temperature, and a cycling factor per cycle, then enter
phases one per line as name, duration hours, temperature in C, cycles,
dormant flag (1 or 0), and dormant factor. Choose Compute mission
profile rate for the profile-weighted rate. It holds no IEC 62380
table data.

### Local library import

For your own component data, use the Local library file source on the
Prediction screen, or the library CSV format generally: one part per
line as name, base rate per million hours, quantity, stress factor,
quality factor. Comment lines starting with # are skipped. On the
desktop, enter the library CSV path and choose Load library. In the
browser demo, Load library opens a file picker. ReliaSoft compiled
libraries are not read; the CSV format is the import route.

## File formats

### Project JSON (reliakit-project-2)

One JSON file saves the whole project: the project name, the selected
item index, and every item with its id, kind, name, and full item
state. The format name written in the file is reliakit-project-2, the
same format the v0.2.0 app used, so project files carry across the
v0.3.0 interface change. Save project and Open project in the toolbar
or the project panel use the project file path shown in the panel. In
the browser demo, Save downloads a file named reliakit-project.json and
Open uses a file picker, then sends the file text to the engine.
ReliaSoft project formats are not read.

### CSV

- Bundled and imported life data CSVs follow the bundled dataset
  layout documented in datasets/README.md. The shared Import data
  controls on the Life Data, Degradation, ALT, Growth, Competing Risks,
  and Warranty screens load a bundled dataset by name or a file you
  supply into the current item.
- FMEA worksheet CSV: a first line with the FMEA type, then one row per
  failure mode. The rows-only export omits the type line.
- FRACAS CSV: one incident per row with id, part, failure mode, hours,
  date, status, and corrective action. Export and import round trip.
- Component library CSV: name, base rate per million hours, quantity,
  stress factor, quality factor, with # comment lines skipped.
- RBD diagrams are JSON, not CSV: the library RBD schema used by the
  command line tool, with an optional additive pos field per leaf for
  canvas coordinates.

On the desktop, file controls take a local path. In the browser demo,
any control that would need a local path is replaced by a file picker,
a download, or a short note on the control itself. The desktop-only
controls in the demo are the CSV file path fields in the shared Import
data controls, the FMEA export file path, the FRACAS CSV file path, the
RBD diagram file path, the Prediction library CSV path, and the project
file path fields. None of them is a dead button in the demo; each one
is labeled and has a working picker or download beside it.

## Keyboard shortcuts

The web interface has four global shortcuts:

- Ctrl+S: save project. On macOS, Cmd+S works the same way.
- Ctrl+O: open project. On macOS, Cmd+O works the same way. Both
  shortcuts replace the browser defaults while the app has focus.
- Ctrl+K: open the quick switcher (Jump to). On macOS, Cmd+K works
  the same way. The switcher works even when focus is in a field.
  Inside the switcher: type to filter, Up and Down move the
  highlight, Enter runs the highlighted entry, and Escape closes it.
- Delete: remove the selected RBD block, the selected FRACAS incident,
  the selected RCM row, or the FMEA worksheet row you last clicked,
  when you are not typing in a field. FMEA rows have no engine-side
  selection state, so the clicked row is tracked by the interface for
  this shortcut; the per-row Delete button always works as well.

Escape also closes the picker, the switcher, and the About view.
Standard text editing keys work in every field. There are no other
global shortcuts.

## Platform notes

- Linux: the desktop app needs the WebKitGTK runtime, which most
  desktop distributions install with their web view packages. If the
  app does not start, install the WebKitGTK runtime package for your
  distribution and try again. Mark the downloaded binary executable
  before running it.
- macOS: the download is a raw binary, not an .app bundle. Mark it
  executable after downloading (for example with chmod +x in a terminal)
  and run it from a terminal. Because the binary is not signed or
  notarized, Gatekeeper may ask you to confirm before the first run.
- Windows: the app is unsigned, so SmartScreen may show a warning on
  the first run. Choose the option to run anyway only when the file
  came from the official v0.3.1 release page.
- Browser demo: the demo needs a current browser with WebAssembly. It
  runs fully in the page; project files, CSV imports, and exports move
  through your browser downloads and file picker, and nothing is
  uploaded.
- Command line: the reliakit command line tool ships beside the app in
  every release and prints JSON results. Run reliakit with a command
  name and --help for its arguments, and reliakit selftest for the
  built-in closed-form checks.
