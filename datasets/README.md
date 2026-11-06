# Bundled sourced datasets

Every file in this folder is transcribed exactly from a documentation
page that was opened and read in full. Nothing here is generated or
invented. Each CSV carries leading comment lines (starting with `#`)
that name the dataset, give the source URL, name the source example,
and record any note about the printed source.

Files whose source prints only part of a dataset are not included.
Attempts that did not produce a shippable file are recorded in the
stream notes, not here.

## Accepted import formats

* Life data: `time,failed` with failed = 1 for a failure and 0 for a
  suspension (right censored). The variant `time,censored` is also
  accepted, with censored = 1 for a suspended unit. Grouped life data
  adds a third column, `count`, the number of units in the row.
* ALT (single stress): `stress,time,failed`. The two-stress file adds
  a fourth column, `stress2`.
* Degradation: `unit,time,value`. The LED accelerated degradation
  file adds a fourth column, `stress`.
* Reliability growth: `time,failures`, one row per failure, where
  time is the cumulative test time at the failure and failures is the
  cumulative failure count (1, 2, 3, ...).
* Warranty (Nevada chart): `ship_period,sales,lag_months,returns`,
  one row per shipment and return lag. The sales column repeats the
  shipment size on each lag row of that shipment.

## Files

### reliasoft_life_data_example.csv

* Origin: ReliaSoft Weibull++ worked example, prototype device life
  test in hours.
* Source: https://help.reliasoft.com/weibull22/content/life_data_analysis_example.htm
* Rows: 10 data rows.
* Columns: `time,failed`. Six failures at 16, 34, 53, 75, 93, 120
  hours and four suspensions at 120 hours.
* Suits: life data analysis.
* Verification: fit verified. The page prints the 2P Weibull MLE fit
  beta = 1.5070, eta = 128.6414. ReliaKit MLE gives beta = 1.5069,
  eta = 128.6430 (test tolerance: beta 0.002, eta 0.005).

### reliasoft_unbiasing_complete.csv

* Origin: ReliaSoft Reliability Basics article on unbiasing Weibull++
  parameters, complete data example. The article states the data were
  generated from a Weibull distribution with beta = 1.5, eta = 100.
* Source: https://help.reliasoft.com/articles/content/hotwire/issue109/relbasics109.htm
* Rows: 10 data rows.
* Columns: `time,failed`. All ten units failed (22, 45, 48, 62, 64,
  74, 90, 134, 189, 192).
* Suits: life data analysis.
* Verification: fit verified. The page prints the 2P Weibull MLE fit
  beta = 1.7255, eta = 103.8609. ReliaKit MLE gives beta = 1.7255,
  eta = 103.8606.

### reliasoft_unbiasing_suspended.csv

* Origin: same article as above, suspended data example. The
  complete data set with every failure after 120 suspended at 120.
* Source: https://help.reliasoft.com/articles/content/hotwire/issue109/relbasics109.htm
* Rows: 10 data rows.
* Columns: `time,failed`. Seven failures (22, 45, 48, 62, 64, 74, 90)
  and three suspensions at 120.
* Suits: life data analysis with suspensions.
* Verification: fit verified. The page prints the 2P Weibull MLE fit
  beta = 1.7670, eta = 100.2278. ReliaKit MLE gives beta = 1.7669,
  eta = 100.2279.

### reliasoft_beta_shape_example.csv

* Origin: ReliaSoft Hot Topics article on the Weibull shape
  parameter, Table 1 failure data, in printed item order.
* Source: https://help.reliasoft.com/articles/content/hotwire/issue110/hottopics110.htm
* Rows: 20 data rows.
* Columns: `time,failed`. Nine failures and eleven suspensions.
* Suits: life data analysis with suspensions.
* Verification: transcribed, fit not reproduced. The page prints
  beta = 0.9812, eta = 73.6036 for the whole group but does not state
  the estimation method. ReliaKit MLE gives beta = 1.2280,
  eta = 65.3757 and ReliaKit rank regression (RRX) gives
  beta = 0.7394, eta = 165.7638, so neither reproduces the printed
  values. The transcription itself is cross-checked: Tables 2 and 3
  on the same page re-list the same 20 rows as subgroups and match
  this file row for row.

### reliasoft_alt_tensile.csv

* Origin: ReliaSoft ALTA worked example, tensile component of a
  landing gear, cycles to failure at three shock loads.
* Source: https://help.reliasoft.com/articles/content/additional_examples/alta/tensile_components.html
* Rows: 15 data rows.
* Columns: `stress,time,failed`. Stress is shock load in kips (73,
  95, 128; use level 50). All 15 units failed.
* Suits: ALT, single stress. The page fits an inverse power law
  lognormal model and prints a B10 life at 50 kips of 11669.1 cycles.

### reliasoft_alt_circuit_boards.csv

* Origin: ReliaSoft ALTA worked example, circuit boards in an
  accelerated life test with temperature as the stress, hours.
* Source: https://help.reliasoft.com/articles/content/additional_examples/alta/circuit_boards_example.html
* Rows: 29 data rows (the grouped counts on the page, expanded to
  one row per unit).
* Columns: `stress,time,failed`. Stress is chamber temperature in
  kelvin (463 and 488). Only rows the page labels Failure are
  failures (15). Units that failed for unrelated, fixture, power
  surge or power regulator reasons, and units removed when the test
  terminated, are suspensions, as the page directs.
* Suits: ALT with suspensions; also usable as a competing risks
  illustration, since the suspension reasons are distinct failure
  causes that the example treats as suspensions.
* Note: the page text says 27 circuit boards, but the printed
  grouped counts sum to 29 units (19 at 463 K, 10 at 488 K). This
  file follows the printed table.

### reliasoft_alt_paper_clips.csv

* Origin: ReliaSoft ALTA worked example, paper clip bend test,
  cycles to failure at three bend angles.
* Source: https://help.reliasoft.com/articles/content/additional_examples/alta/paper_clip_example.html
* Rows: 18 data rows.
* Columns: `stress,time,failed`. Stress is bend angle in degrees
  (45 is the use level, 90 and 180 are accelerated). Six clips per
  angle, all failed.
* Suits: ALT, single stress. The page fits an inverse power law
  lognormal model and prints an MTTF at 45 degrees of 71.375467
  cycles.

### reliasoft_alt_two_stress.csv

* Origin: ReliaSoft ALTA two-stress worked example, hours to failure
  at combinations of temperature and voltage.
* Source: http://help.synthesisplatform.net/weibull_alta8/alta_two-stress_example.htm
* Rows: 12 data rows.
* Columns: `stress,time,failed,stress2`. Stress is temperature in
  kelvin (348, 378), stress2 is voltage in volts (3, 5). Use level
  is 328 K and 2 V. All 12 units failed.
* Suits: ALT with two stresses (the page uses the
  temperature-nonthermal Weibull model).

### reliasoft_degradation_turbine_blades.csv

* Origin: ReliaSoft Weibull++ degradation worked example, turbine
  blade crack length in mm against cycles.
* Source: http://help.synthesisplatform.net/weibull_alta8/weibull_degradation_analysis_example.htm
* Rows: 15 data rows (3 units x 5 inspections).
* Columns: `unit,time,value`. Unit 1, 2, 3 are the printed units A,
  B, C. Time is in thousands of cycles, exactly as printed (100 to
  500). Value is crack length in mm. Failure is a crack length of
  30 mm or more.
* Suits: degradation analysis. The page uses an exponential
  degradation model and prints a 2P Weibull MLE fit on the projected
  failure times of beta = 8.0722, eta = 541.4229 (eta in thousands of
  cycles).

### reliasoft_degradation_mosfet.csv

* Origin: ReliaSoft Weibull++ reference example for a power
  regression degradation model. The page states the data are from
  Example 8.1 on page 336 of Life Cycle Reliability Engineering by
  Guangbin Yang (2007): percent transconductance degradation of five
  MOS field-effect transistor units.
* Source: https://help.reliasoft.com/articles/content/reference_examples/weibull/degradation_data_analysis_with_a_power_regression_model.html
* Rows: 175 data rows (5 units x 35 inspection times, 100 to 40000).
* Columns: `unit,time,value`. Value is percent degradation. Failure
  criterion is degradation of 15 percent or more.
* Suits: degradation analysis. The page prints pseudo failure times
  of 17602, 31674, 76228, 139698, 381542 and a lognormal fit with
  Ln-Mean = 11.214, Ln-Std = 1.085.

### reliasoft_alt_degradation_led.csv

* Origin: ReliaSoft accelerated degradation worked example, LED
  light intensity in mcd under two current levels.
* Source: https://help.reliasoft.com/weibull22/content/alta_degradation_analysis_example.htm
* Rows: 50 data rows (10 units x 5 inspections at 50 hour
  intervals).
* Columns: `unit,time,value,stress`. Units 1 to 5 are the printed
  units A1 to A5 at 35 mA, units 6 to 10 are B1 to B5 at 40 mA. Use
  level is 28 mA. Critical degradation level is 50 mcd.
* Suits: accelerated degradation analysis (degradation with a stress
  column). The page prints a B10 life at use level of 1109.9328
  hours.

### reliasoft_growth_crow_amsaa.csv

* Origin: ReliaSoft reliability growth reference book example,
  Failure Times Crow-AMSAA Example: a prototype system tested for
  300 hours with 27 failures. This ReliaWiki page prints the data as
  a table. The current help.reliasoft.com version of the related
  example set shows its data only in screenshots, so this printed
  table version is the one transcribed.
* Source: https://reliawiki.com/index.php?title=Failure_Times_Crow-AMSAA_Example&oldid=48924
* Rows: 27 data rows.
* Columns: `time,failures`. Time is the cumulative failure time in
  hours, sorted; failures is the cumulative failure count, 1 to 27.
  The test was time terminated at 300 hours.
* Suits: reliability growth (Crow-AMSAA). The page prints an
  instantaneous MTBF lower one-sided 90 percent bound at 300 hours
  of 10.8170 hours against a 10 hour goal.

### reliasoft_growth_test_fix_test.csv

* Origin: ReliaSoft RGA reference example for test-fix-test data.
  The page gives the reference case as Crow (1981), Confidence
  Interval Procedures for the Weibull Process.
* Source: https://help.reliasoft.com/articles/content/reference_examples/rga/test-fix-test_data_reference_example.html
* Rows: 23 data rows.
* Columns: `time,failures`. Time is the cumulative failure time in
  hours; failures is the cumulative failure count, 1 to 23. The
  test was time terminated at 500 hours.
* Suits: reliability growth (Crow-AMSAA). The page prints the book
  results Beta = 0.413, Lambda = 1.769, DMTBF = 52.7.

### reliasoft_warranty_nevada.csv

* Origin: ReliaSoft life data analysis reference, Warranty Data
  Analysis, Nevada Chart Format Calculations Example: shipments in
  June, July and August 2010 and warranty returns through
  September 2010.
* Source: https://help.reliasoft.com/reference/life_data_analysis/lda/warranty_data_analysis.html
* Rows: 6 data rows (one per shipment and observed return lag).
* Columns: `ship_period,sales,lag_months,returns`. Ship period 1 is
  June 2010 (100 shipped), 2 is July 2010 (140 shipped), 3 is
  August 2010 (150 shipped). Total shipped 390, total returns 21.
* Suits: warranty analysis (Nevada chart). Converted to life data
  per the page text: at 1 month, 9 failures and 146 suspensions; at
  2 months, 7 failures and 134 suspensions; at 3 months, 5 failures
  and 89 suspensions.

### reliasoft_warranty_time_to_failure.csv

* Origin: ReliaSoft life data analysis reference, Warranty Data
  Analysis, Times-to-Failure Format Warranty Analysis example.
* Source: https://help.reliasoft.com/reference/life_data_analysis/lda/warranty_data_analysis.html
* Rows: 4 data rows (grouped).
* Columns: `time,failed,count`. Count is the printed Number in
  State: 2 failures at 100 hr, 3 at 125 hr, 5 at 175 hr, and 1500
  suspensions at 200 hr (1510 units in total, 10 failures).
* Suits: warranty analysis in time-to-failure format; also grouped
  life data. The printed fit is rank regression (RRX), not MLE:
  beta = 3.199832, eta = 814.293442. The future sales printed on the
  page (500 at 200 hr, 400 at 300 hr, 100 at 500 hr) are forecast
  inputs and are not part of this data table.

### smrd_bearingcage.csv

* Origin: classic textbook dataset, the bearing cage fracture test
  from Meeker and Escobar (1998), Statistical Methods for
  Reliability Data, as distributed in the SMRD R package (data
  originally from Abernethy et al. 1983). Field service times for
  1703 ball bearing assemblies, grouped.
* Source: https://rdrr.io/github/Auburngrads/SMRD/src/inst/test_objs/wqmmlesss/bearingcage.R
  (the SMRD source file that prints the full hours, event and count
  vectors; documentation page:
  https://rdrr.io/github/Auburngrads/SMRD.data/man/bearingcage.html)
* Rows: 25 data rows (grouped).
* Columns: `time,failed,count`. In the source, the event factor
  levels are 1 = Censored and 2 = Failed; they are converted here to
  failed = 1 for Failed and 0 for Censored. Count is the number of
  assemblies in the row.
* Suits: life data analysis with heavy censoring (grouped). Totals
  match the documentation: 1703 assemblies, 6 failures at 230, 334,
  423, 990, 1009 and 1510 hours, and 1697 censored.

## Verification

`tests/datasets_tests.rs` loads every file above and asserts the
documented row count and column sums, so a transcription slip fails
the test suite. It also fits the three life data examples whose
pages print MLE parameters (life data example, unbiasing complete,
unbiasing suspended) with `reliakit::distributions::fit_mle` and
asserts agreement with the printed beta and eta within beta 0.002
and eta 0.005. Run with:

    cargo test --release --test datasets_tests
