//! ReliaKit engine: the host-independent half of the v0.3.0 app.
//!
//! `work` is the proven workspace engine moved unchanged from the
//! Slint GUI (project model in the reliakit-project-2 format, every
//! screen's fields and computations, the six data flows, examples
//! 1 to 8, bundled dataset and MIL-HDBK-217F handling).
//!
//! `series` ports the chart series computations from the old
//! plotters charts as pure data. `handle` is the JSON dispatcher
//! both hosts use: the Tauri app through its `dispatch` command and
//! the browser demo through the WASM exports. See API.md for the
//! full contract.

#![recursion_limit = "512"]

pub mod handle;
pub mod series;
pub mod work;

pub use handle::handle;
pub use work::{project_from_value, project_to_value, ItemKind, ProjectItem, Work, BUNDLED_CSV, MIL217_FILES};

/// Text of one bundled dataset, by file name (one of BUNDLED_CSV).
///
/// The CSV files are embedded at compile time so hosts without a
/// filesystem (the WASM browser demo) return the same bytes as the
/// desktop build reads from the datasets directory. Returns None
/// for an unknown name.
pub fn bundled_csv_text(name: &str) -> Option<&'static str> {
    let text = match name {
        "reliasoft_alt_circuit_boards.csv" => include_str!("../../datasets/reliasoft_alt_circuit_boards.csv"),
        "reliasoft_alt_degradation_led.csv" => include_str!("../../datasets/reliasoft_alt_degradation_led.csv"),
        "reliasoft_alt_paper_clips.csv" => include_str!("../../datasets/reliasoft_alt_paper_clips.csv"),
        "reliasoft_alt_tensile.csv" => include_str!("../../datasets/reliasoft_alt_tensile.csv"),
        "reliasoft_alt_two_stress.csv" => include_str!("../../datasets/reliasoft_alt_two_stress.csv"),
        "reliasoft_beta_shape_example.csv" => include_str!("../../datasets/reliasoft_beta_shape_example.csv"),
        "reliasoft_degradation_mosfet.csv" => include_str!("../../datasets/reliasoft_degradation_mosfet.csv"),
        "reliasoft_degradation_turbine_blades.csv" => include_str!("../../datasets/reliasoft_degradation_turbine_blades.csv"),
        "reliasoft_growth_crow_amsaa.csv" => include_str!("../../datasets/reliasoft_growth_crow_amsaa.csv"),
        "reliasoft_growth_test_fix_test.csv" => include_str!("../../datasets/reliasoft_growth_test_fix_test.csv"),
        "reliasoft_life_data_example.csv" => include_str!("../../datasets/reliasoft_life_data_example.csv"),
        "reliasoft_unbiasing_complete.csv" => include_str!("../../datasets/reliasoft_unbiasing_complete.csv"),
        "reliasoft_unbiasing_suspended.csv" => include_str!("../../datasets/reliasoft_unbiasing_suspended.csv"),
        "reliasoft_warranty_nevada.csv" => include_str!("../../datasets/reliasoft_warranty_nevada.csv"),
        "reliasoft_warranty_time_to_failure.csv" => include_str!("../../datasets/reliasoft_warranty_time_to_failure.csv"),
        "smrd_bearingcage.csv" => include_str!("../../datasets/smrd_bearingcage.csv"),
        _ => return None,
    };
    Some(text)
}

/// Text of one MIL-HDBK-217F library file, by file name (one of
/// MIL217_FILES), embedded the same way as the bundled datasets.
/// Returns None for an unknown name.
pub fn mil217_text(name: &str) -> Option<&'static str> {
    let text = match name {
        "capacitors.json" => include_str!("../../libraries/mil217/capacitors.json"),
        "circuit_breakers.json" => include_str!("../../libraries/mil217/circuit_breakers.json"),
        "coils.json" => include_str!("../../libraries/mil217/coils.json"),
        "connections.json" => include_str!("../../libraries/mil217/connections.json"),
        "connector_sockets.json" => include_str!("../../libraries/mil217/connector_sockets.json"),
        "connectors.json" => include_str!("../../libraries/mil217/connectors.json"),
        "elapsed_time_meters.json" => include_str!("../../libraries/mil217/elapsed_time_meters.json"),
        "electronic_filters.json" => include_str!("../../libraries/mil217/electronic_filters.json"),
        "fuses.json" => include_str!("../../libraries/mil217/fuses.json"),
        "interconnection_assemblies_pth.json" => include_str!("../../libraries/mil217/interconnection_assemblies_pth.json"),
        "miscellaneous_parts.json" => include_str!("../../libraries/mil217/miscellaneous_parts.json"),
        "quartz_crystals.json" => include_str!("../../libraries/mil217/quartz_crystals.json"),
        "relays_mechanical.json" => include_str!("../../libraries/mil217/relays_mechanical.json"),
        "relays_solid_state.json" => include_str!("../../libraries/mil217/relays_solid_state.json"),
        "resistors.json" => include_str!("../../libraries/mil217/resistors.json"),
        "switches.json" => include_str!("../../libraries/mil217/switches.json"),
        "synchros_and_resolvers.json" => include_str!("../../libraries/mil217/synchros_and_resolvers.json"),
        "transformers.json" => include_str!("../../libraries/mil217/transformers.json"),
        _ => return None,
    };
    Some(text)
}
