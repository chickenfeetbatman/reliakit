# ReliaKit engine API

The engine crate (`reliakit-engine`) is the host-independent half of
ReliaKit v0.3.0. Both hosts own one `Work` value and talk to it
through one JSON dispatcher (`handle::handle` for a parsed request,
`handle::handle_str` for JSON text):

- The Tauri desktop app exposes it as the single command
  `dispatch`. It accepts the request as a JSON string in the
  `req` argument or as a parsed JSON value (or string) in the
  `request` argument, and returns the full state as a JSON
  string.
- The WASM browser demo exposes it as `rk_dispatch(ptr, len)`,
  with `rk_init()` returning the initial state.

Project file IO and CSV file IO stay with the host side of the
work engine: path fields (for example `project-path`,
`CsvUi.path`, `RbdUi.path`) are read and written by the engine
against a real filesystem on desktop. Hosts without a filesystem
use the `project_open_data` and `csv_load_text` actions below,
and every response carries a full project snapshot.

## Request

```json
{ "action": "life-fit", "fields": { "LifeUi.times": "1200, 2400, 3100c", "LifeUi.t": "1000" } }
```

- `action` (string, required in practice): one of the action
  strings below. A missing or empty action, or `state`, `init`,
  `get_state`, `noop`, just returns the current state.
- `fields` (object, optional): field values for the current
  screen, mirroring the pull step of the old Slint glue. Keys the
  engine does not know are ignored. Top-level request keys other
  than `action` and `fields` are also read as fields; entries in
  `fields` win.

### Field naming

Key spellings are normalized before the pull: dots and hyphens
become underscores, and the suffix after the first dot is also
registered, so `life.times`, `life_times`, and `times` reach the
same pull entry, and `pred.p217_file` reaches the MIL-HDBK-217F
file field. Beyond that, three key spellings are accepted for
every field, shown here for the Life Data time input:

- Global-qualified Slint name: `LifeUi.times`
- Work field name: `life_times`
- Bare Slint property name, read on the current screen: `times`

Property names keep the exact Slint spelling from
`gui/ui/state.slint`, including hyphens (`dist-index`,
`use-stress`, `ed-mtbf`). Values may be JSON strings, numbers, or
booleans; the engine stores strings, so numbers and booleans are
converted with their plain JSON spelling. Only fields present in
the map are applied: a host may send a full form snapshot or just
the fields that changed.

Enum fields (distribution, model, topology, and the other option
lists) accept either the value itself (`"life_dist": "weibull2"`)
or an index into the same list the Slint UI used, under the
matching `*-index` key (`"LifeUi.dist-index": 0`). The response
carries both spellings back as `<name>-index`, the value under its
plain name where the Slint global had a string, and a
`<name>-options` list where the front end needs the choices.

Dynamic per-parameter fields travel as map entries of their own,
exactly like the old field-changed callback:

- `qcp:<param>` sets one QCP parameter (for example `qcp:eta`).
- `p217:<key>` sets one MIL-HDBK-217F field; whether it lands in
  the named or numeric map follows the loaded category, as before.

### Per-screen fields

Shell (every screen): `AppState.project-name`,
`AppState.project-path`, `AppState.kind-index` or `new_kind`,
`AppState.new-name`, `AppState.rename-name`, `CsvUi.pick-index`
or `csv_pick`, `CsvUi.path`.

- LifeUi: `times`, `dist-index` or `dist`, `rank`, `t`,
  `show-prob`, `target-index` or `flow_target`, `block-index`.
- LimitedUi: `groups`.
- McmcUi: `times`, `dist-index` or `dist`, `draws`, `warmup`,
  `chains`, `seed`, `t`.
- DegUi: `units`, `threshold`, `model-index` or `model`,
  `dist-index` or `dist`, `increasing`.
- AltUi: `data`, `model-index` or `model`, `use-stress`, `t`.
- GrowthUi: `times`.
- CompUi: `data`, `dist-index` or `dist`, `t`.
- RbdUi: `t`, `horizon`, `topology-index` or `topology`, `k`,
  `ed-name`, `ed-dist-index` or `ed_dist`, `ed-params`, `ed-mtbf`,
  `ed-mttr`, `ed-units`, `ed-dormancy`, `ed-switch`, `nb-name`,
  `nb-dist-index` or `nb_dist`, `nb-params`, `nb-mtbf`, `nb-mttr`,
  `nb-mode-index` or `nb_mode`, `nb-dormancy`, `json`, `path`.
- FtUi: `json`.
- FmeaUi: `type-index` or `type`, `fm-item`, `fm-mode`, `fm-sev`,
  `fm-occ`, `fm-det`, `fm-alpha`, `fm-beta`, `fm-lambda`,
  `fm-hours`, `dv-id`, `dv-ref`, `dv-test`, `dv-sample`,
  `dv-criteria`, `csv`, `export-path`, `rcm-row`.
- WarUi: `sales`, `returns`, `current`, `horizon`.
- PredUi: `source-index` or `source`, `life-dist-index` or
  `life_dist`-style value key `pred_life_dist`, `life-params`,
  `life-t`, `parts`, `t`, `lib-path`, `p217-file-index` or
  `pred217_file`, `p217-part-index` or `p217_part`,
  `p217-env-index` or `p217_env`, `alloc-target`, `alloc-n`,
  `alloc-weights`, `der-name`, `der-operating`, `der-rated`,
  `der-max`, the `sr-*` clean-room inputs, and `mp-base`, `mp-ea`,
  `mp-ref`, `mp-cyc`, `mp-phases`.
- DoeUi: `k`, `y`.
- FlowUi: `json`, `sims`, `seed`.
- MarkovUi: `new-state`, `start-index` or `mk_start`,
  `absorbing-index` or `mk_absorbing`, `time`, and for the
  selected transition `tr-from`, `tr-to`, `tr-rate`.
- QcpUi: `dist-index` or `dist`, `t`, `bx`, plus `qcp:<param>`
  entries.
- RcmUi (selected row editor): `ed-asset`, `ed-function`,
  `ed-mode`, `ed-consequence-index` or `consequence`,
  `ed-monitorable`, `ed-age`, `ed-restorable`, `ed-rate`,
  `ed-alpha`, `ed-beta`, `ed-hours`.
- FracasUi: `total-hours`, `path`, and for the selected incident
  `ed-id`, `ed-part`, `ed-mode`, `ed-hours`, `ed-date`,
  `ed-status-index` or `ed_status`, `ed-action`.

## Response

Every request returns the full UI state as one JSON object:

- `status`: the status line text (also `AppState.status`).
- `headline`: the headline text (also `AppState.headline`).
- `screen`: the current screen key (`start`, `life`, `limited`,
  `mcmc`, `degradation`, `alt`, `growth`, `competing`, `rbd`,
  `faulttree`, `fmea`, `warranty`, `prediction`, `doe`,
  `flowchart`, `markov`, `qcp`, `rcm`, `fracas`).
- `AppState`, `Actions`, `CsvUi`, and one object per screen
  global from `gui/ui/state.slint` (`LifeUi`, `LimitedUi`,
  `McmcUi`, `DegUi`, `AltUi`, `GrowthUi`, `CompUi`, `RbdUi`,
  `FtUi`, `FmeaUi`, `WarUi`, `PredUi`, `DoeUi`, `FlowUi`,
  `MarkovUi`, `QcpUi`, `RcmUi`, `FracasUi`). Keys inside each
  object are the exact Slint property names. Result tables are
  arrays of `{"label": string, "value": string}` (the old PairRow)
  under `results` and similar keys; grids are arrays of
  `{"cells": [string, ...]}` (the old Cells) under `rows` and
  similar keys. Where Slint held an image, the JSON holds `icon`
  as the icon name string. Extra keys beyond the Slint contract
  (value spellings of enums, `*-options` lists, `RbdUi.spec`) are
  additive and safe to ignore.
- `AppState.items`: array of `{"id", "kind", "kind-label", "name",
  "icon"}` for the project sidebar. `AppState.example-rows` and
  `AppState.dataset-rows` are arrays of rows of card objects
  `{"title", "sub", "tag", "icon", "action-id"}` for the start
  page; send a card's `action-id` back with `card_action`.
- `chart`: `{"title": string, "has-chart": bool, "series": [...]}`:
  the current screen's chart as pure data (see Series below).
  Chart screens also carry the same data under their own `series`,
  `has-chart`, and (LifeUi) `chart-title` keys.
- `series`: the same series array, at the top level.
- `app`, `csv`, `life`, `limited`, `mcmc`, `deg`, `degradation`,
  `alt`, `growth`, `comp`, `competing`, `rbd`, `ft`, `faulttree`,
  `fmea`, `war`, `warranty`, `pred`, `prediction`, `doe`, `flow`,
  `flowchart`, `markov`, `qcp`, `rcm`, `fracas`: additive alias
  copies of the matching globals with snake_case keys, plus
  top-level snake_case mirrors of the AppState summary fields
  (`items`, `selected_index`, `project_name`, `project_path`,
  `current_title`, `kind_index`, `new_name`, `rename_name`,
  `item_count`, `example_rows`, `dataset_rows`, `csv_files`).
  Hosts may read either spelling; the Slint-named objects are the
  canonical ones.
- `project`: the project snapshot as a parsed JSON value in the
  `reliakit-project-2` format (the `project_to_value` output).
- `project_json`: the same snapshot as a pretty-printed string,
  ready to save or download.

### Series

Each series is `{"name": string, "kind": "line" | "points",
"points": [[x, y], ...]}`. The mapping from the old plotters
charts: Life Data gives the fitted reliability curve (121 points)
or, in probability mode, the `y = x` line plus the failure points;
Degradation gives the threshold line, each unit's raw points, and
each unit's fitted path; Growth gives the Crow-AMSAA expected
line plus observed failure points; ALT gives the reliability at
use stress line; Markov gives one transient probability line per
state.

## v0.3.2 correctness additions

- Precision: stored numeric text uses shortest round-trip
  formatting (`fmt_num`, Rust f64 Display). Text that is stored
  in an item field and parsed back later (life times, example
  and dataset generator text, CSV imported text, FRACAS hours
  sent to Life Data, degradation and ALT text) keeps fractional,
  very small, and very large values exactly. Display-only result
  rows keep their fixed-decimal formatting.
- Structured validation: every response carries a top-level
  `error` key. It is null on success, or an object
  `{"field": string, "token": string, "reason": string}` naming
  the offending field and token. The `status` text carries the
  same human-readable message. On a validation error the action
  does not run the calculation and does not overwrite prior
  stored results. Invalid numeric tokens, invalid mission times,
  missing CSV cells, malformed counts, invalid censoring flags,
  negative times, fractional values in integer fields, the
  tokens NaN and inf and infinity, overflow such as 1e999, and
  empty required fields all produce this error.
- Fit status: screens whose run produces a fit (Life Data,
  Limited Data, Competing Risks, Degradation, ALT) carry
  `fit-status` (one of success, success_with_warnings,
  boundary_or_degenerate, failed), `fit-status-note` (string),
  and `fit-warnings` (array of strings) in their state object.
  Life Data wizard rows carry a `status` value per distribution;
  failed fits rank last with their reason shown. A failed fit
  has `has-fit` false. A boundary_or_degenerate fit with finite
  parameters has `has-fit` true and its headline shows the
  status note. Result rows never report NaN or inf as a value;
  unavailable values read `unavailable: <reason>`.
- Outdated results: each computing item stores a fingerprint of
  its input fields at compute time. When the inputs change, the
  screen state object and the top level carry `outdated`: true
  (false again after a recompute). The top level also carries a
  `warning` key, null normally or
  `{"kind": "outdated", "message": string}`. Data-flow send
  actions (flow-rbd, flow-qcp, flow-pred, fmea-to-rcm,
  flow-fracas-life, wizard-use) refuse when the source result is
  outdated: they do not perform the send and set the outdated
  warning. They proceed when the request fields include
  `confirm_outdated` with value true (or the string "true").
- `duplicate_item` action: deep-copies the currently selected
  project item (inputs, spec and diagram state, stored results,
  and the result fingerprint), assigns a fresh item id, names it
  "<original name> copy" (or " copy 2" and so on when that name
  exists), selects the copy, and returns the full state.
- Example naming: an example loaded with `card_action` is named
  "Example: <example title>", for example
  "Example: RBD with cold standby". Dataset card loads keep the
  dataset file name in the item name.
- Chart title: the Life Data probability mode chart title is
  exactly "Empirical vs fitted probability". The reliability
  mode title stays "Reliability curve".

## Actions

Shell actions (the old Actions callbacks; hyphen and underscore
spellings both work):

| Action | Effect | Fields used |
| --- | --- | --- |
| `state`, `init`, `get_state`, `noop`, empty | No change, returns state | any (pulled) |
| `go_start` | Show the start page (deselect) | |
| `select_item` | Select a project item | `index` |
| `add_item` | Add an item from the panel fields | `kind-index` or `new_kind`, `new-name` |
| `duplicate_item` | Duplicate the selected item (see v0.3.2 additions) | |
| `remove_item` | Remove the selected item | |
| `rename_item` | Rename the selected item | `rename-name` |
| `new_project` | Start a new empty project | |
| `save_project` | Write the project to `project-path` (desktop), snapshot in response | `project-path` |
| `open_project` | Read the project from `project-path` (desktop) | `project-path` |
| `project_open_data` | Open a project from JSON text, v2 or legacy format | `text` (also `data`, `json`, `project`, `content`) |
| `card_action` | Start page card: load an example or dataset item | `id` (the card `action-id`, `example:<n>` or `dataset:<file>`) |
| `csv_bundled` | Load the picked bundled dataset into the current item | `pick-index` or `csv_pick` |
| `csv_file` | Load the CSV file at `CsvUi.path` into the current item (desktop) | `CsvUi.path` |
| `csv_load_text` | Load CSV text into the current item | `text` (also `data`, `csv`, `content`) |

Screen actions: send the name either as the top-level `action`
or as `action: "act"` with the name in `fields.name`. These are
the exact strings the Slint UI sent:

- Life Data: `life-fit`, `life-wizard`, `flow-rbd`, `flow-qcp`,
  `flow-pred`, `flow-target` (no-op marker from the target
  pickers; the pull step does the work).
- Limited Data: `limited-run`.
- MCMC: `mcmc-run`.
- Degradation: `deg-run`.
- ALT: `alt-run`.
- Growth: `growth-run`.
- Competing Risks: `comp-run`.
- RBD: `rbd-avail`, `rbd-importance`, `rbd-topology`,
  `rbd-apply-props`, `rbd-duplicate`, `rbd-delete`, `rbd-undo`,
  `rbd-add-form`, `rbd-load-json`, `rbd-save-file`,
  `rbd-load-file`.
- Fault Tree: `ft-run`.
- FMEA: `fmea-add-row`, `fmea-to-rcm`, `fmea-add-dvpr`,
  `fmea-export-ws`, `fmea-export-rows`, `fmea-import`,
  `fmea-clear`, `fmea-export-cp`, `fmea-export-dvpr`.
- Warranty: `war-run`.
- Prediction: `pred-life`, `pred-run`, `pred-lib-load`,
  `pred217-load`, `pred217-compute`, `pred-alloc`, `pred-derate`,
  `pred-clean`, `pred-mission`.
- DOE: `doe-run`.
- Flowchart: `flow-run`.
- Markov: `mk-preset`, `mk-run`, `mk-add-state`, `mk-add-trans`,
  `mk-apply-trans`, `mk-del-trans`.
- QCP: `qcp-dist` (reset parameters for the chosen distribution),
  `qcp-run`.
- RCM: `rcm-add`, `rcm-refresh`, `rcm-remove`.
- FRACAS: `fracas-add`, `fracas-refresh`, `fracas-remove`,
  `fracas-export-csv`, `fracas-import-csv`, `flow-fracas-life`.

Row actions: send the name as the top-level `action` (or as
`action: "row_act"` with the name in `fields.name`) and the row
index in `fields.index`:

- `wizard-use` (Life Data wizard row), `fmea-rcm-row` (send that
  FMEA row to RCM), `fmea-del`, `rcm-select`, `fracas-select`,
  `mk-select`, `mk-del-state`.

Text import actions for hosts without a filesystem, beyond
`project_open_data` and `csv_load_text`:

- `pred_lib_load_text`: parse `fields.text` as a prediction
  library CSV into the Prediction item (the file-picker form of
  `pred-lib-load`).
- `fracas_import_text`: parse `fields.text` as a FRACAS CSV into
  the FRACAS item (the file-picker form of `fracas-import-csv`).
- `rbd-load-json` with `fields.text` (or `file_text`): use that
  text as the RBD diagram JSON, then load it as usual.

Field action: `action: "field_changed"` with `fields.id` of the
form `qcp:<param>` or `p217:<key>` and `fields.value`. Sending
`qcp:` or `p217:` entries directly in `fields` does the same
thing on any action.

RBD canvas actions:

| Action | Effect | Fields used |
| --- | --- | --- |
| `rbd_arm` | Toggle a palette tool | `name` |
| `rbd_place` | Place the armed palette item | `x`, `y` (pointer position) |
| `rbd_press` | Select a block and load its editor | `index` |
| `rbd_drag` | Move a block | `index`, `x`, `y`, optional `grab_x`, `grab_y`. Without grab offsets, `x` and `y` are the new top-left corner; with them, they are the pointer position. |
| `rbd_release` | End a drag (no-op beyond the pull and push cycle) | `index` |

## Helpers for hosts

- `work::project_to_value(project_name, items, selected)` and
  `work::project_from_value(value)`: the pure project format
  conversion (format `reliakit-project-2`; v1 files open through
  the legacy branch of `open_project` and `project_open_data`).
- `bundled_csv_text(name)`: the text of one of the 16 bundled
  datasets (`work::BUNDLED_CSV`), embedded at compile time so the
  browser demo returns the same bytes as the desktop files.
- `mil217_text(name)`: the text of one of the 18 MIL-HDBK-217F
  library files (`work::MIL217_FILES`), embedded the same way.
  The dispatcher uses it as a fallback for `pred217-load` and the
  embedded CSVs as a fallback for `csv_bundled` and dataset
  cards, so the browser demo needs no filesystem for bundled
  data. Path-based loads (project files, user CSVs, the local
  prediction library, FMEA and FRACAS file IO, RBD files) still
  need a real filesystem and are desktop-only.
- `handle::new_work()`: a `Work` in the desktop app's start
  state (default project, one Life Data item, start page shown).

## WASM exports

The wasm crate exports raw FFI, no wasm-bindgen: `rk_alloc(len)`,
`rk_free(ptr, len)`, `rk_init()`, and `rk_dispatch(ptr, len)`.
Allocations are registered by pointer, so `rk_free` releases the
registered allocation (the length argument may be omitted by
callers that tracked sizes themselves) and is a no-op on any
other pointer, including response buffers. `rk_dispatch` with a
nonzero length reads that many bytes of JSON at the pointer;
with length 0 it reads a length-prefixed buffer instead (4
little-endian length bytes, then the JSON), which is the form
the demo page sends. Responses from `rk_init` and `rk_dispatch`
are length-prefixed the same way and are owned by the wasm side
in a thread_local static, valid until the next call.
