# MIL-HDBK-217F Notice 2 library

Part stress prediction models transcribed from MIL-HDBK-217F
Notice 2 (28 February 1995), a public US government handbook.
Every number in these files comes from the handbook PDF.

Sources used for the transcription:

- The Notice 2 change pages, downloaded from Everyspec
  (MIL-HDBK-217F_NOTICE-2.pdf, 13,062,097 bytes, 80 PDF pages).
  Notice 2 replaced Sections 9 and 10 with condensed models
  (Sections 9.1 and 10.1) and issued revised pages for Sections
  5, 6, 7, 11, 12, 13, 14, 15, 16 and 17. Those categories are
  transcribed from this file.
- The base handbook, MIL-HDBK-217F (2 December 1991), via
  Quanterion (MIL-HDBK-217F_base_Quanterion.pdf, 27,858,510 bytes,
  205 PDF pages). Sections 19, 21, 22 and 23 were not revised by
  Notice 1 or Notice 2, so their base pages are the pages in force
  under Notice 2. Those categories are transcribed from this file,
  and each JSON file names its source PDF in the source_pdf field.

PDF page numbers below are page numbers in the source PDF named in
each file. Handbook page numbers are the printed page numbers.

## Category files

| File | Section | Handbook pages | PDF pages | Coverage |
| --- | --- | --- | --- | --- |
| resistors.json | 9.1 Resistors | 9-1 to 9-3 | 32 to 34 (Notice 2) | full |
| capacitors.json | 10.1 Capacitors | 10-1 to 10-6 | 35 to 40 (Notice 2) | full |
| transformers.json | 11.1 Inductive Devices, Transformers | 11-1 | 41 (Notice 2) | full |
| coils.json | 11.2 Inductive Devices, Coils | 11-3 | 43 (Notice 2) | full |
| synchros_and_resolvers.json | 12.2 Rotating Devices, Synchros and Resolvers | 12-4 | 48 (Notice 2) | full |
| elapsed_time_meters.json | 12.3 Rotating Devices, Elapsed Time Meters | 12-5 | 49 (Notice 2) | full |
| relays_mechanical.json | 13.1 Relays, Mechanical | 13-1, 13-2 | 50, 51 (Notice 2) | full |
| relays_solid_state.json | 13.2 Relays, Solid State and Time Delay | 13-3 | 52 (Notice 2) | full |
| switches.json | 14.1 Switches | 14-1, 14-2 | 53, 54 (Notice 2) | full |
| circuit_breakers.json | 14.2 Switches, Circuit Breakers | 14-3 | 55 (Notice 2) | full |
| connectors.json | 15.1 Connectors, General | 15-1, 15-2 | 56, 57 (Notice 2) | full |
| connector_sockets.json | 15.2 Connectors, Sockets | 15-3 | 58 (Notice 2) | full |
| interconnection_assemblies_pth.json | 16.1 Interconnection Assemblies with Plated Through Holes | 16-1 | 59 (Notice 2) | full |
| connections.json | 17.1 Connections | 17-1 | 63 (Notice 2) | full |
| quartz_crystals.json | 19.1 Quartz Crystals | 19-1 | 170 (base) | full |
| electronic_filters.json | 21.1 Electronic Filters, Non-Tunable | 21-1 | 172 (base) | full |
| fuses.json | 22.1 Fuses | 22-1 | 173 (base) | full |
| miscellaneous_parts.json | 23.1 Miscellaneous Parts | 23-1, 23-2 | 174, 175 (base) | full |

Notes on individual files:

- resistors.json and capacitors.json hold the condensed unified
  models that Notice 2 substituted for the Revision F Sections 9
  and 10. The detailed per subtype models of the base handbook
  (for example the old Section 9.1 fixed composition resistor
  model) were superseded by Notice 2 and are not included.
- interconnection_assemblies_pth.json uses the printed Section
  16.1 equation, which is not a plain product of pi factors. The
  Rust function reliakit::standards217::pth_failure_rate
  evaluates it.
- miscellaneous_parts.json holds fixed failure rates rather than
  a lambda_b product model, as printed in Section 23.1.
- synchros_and_resolvers.json uses the printed lambda_b table.
  The formula printed with that table does not reproduce the
  table values, so the table is what the library stores.

## Categories not built

- Section 5, Microcircuits: partial, not built. The Notice 2 PDF
  contains only the replacement handbook pages 5-3, 5-4, 5-5,
  5-6, 5-9, 5-10, 5-23 and 5-24. The pi_T table (page 5-13), the
  C2 table (page 5-14) and the pi_E, pi_L and pi_Q tables
  (page 5-15) are not in the Notice 2 PDF, and the C1 complexity
  tables run for many pages, so no complete microcircuit model
  can be computed from this PDF alone.
- Section 6, Discrete Semiconductors: partial, not built. The
  Notice 2 PDF contains only handbook pages 6-1 and 6-2; the
  models continue on pages that Notice 2 did not reissue.
- Section 7, Tubes: skipped, per the stream scope. Only
  replacement pages are in the Notice 2 PDF.
- Section 8, Lasers: skipped, per the stream scope. Not in the
  Notice 2 PDF.
- Section 12.1, Motors: not built. It is a Weibull characteristic
  life model (bearing and winding life tables and constants), not
  a lambda_b times pi factors model, so it does not fit this
  library's data format.
- Section 16.2, Surface Mount Technology: not built. It is a
  thermal cycling fatigue (Weibull) calculation, not a lambda_b
  times pi factors model.
- Section 18, Meters: skipped, per the stream scope.
- Section 20, Lamps: skipped, per the stream scope.
- Appendix A, Parts Count: not built. The parts count method is
  outside the part stress scope of this library.
