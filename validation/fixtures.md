# ReliaKit validation fixtures for v0.3.2

This file is the fixture record for `tests/validation_tests.rs`.
Every case in the test file names the fixture here. Every expected
value has an independent anchor: a published or closed form value, a
hand computation recorded here, or a second computation path written
inside the test itself. No expected value is copied from the output
of the function it checks.

Conventions used in every fixture unless the fixture says otherwise:

- Units are hours for life times. Rescaling fixtures state their
  factor explicitly.
- Censoring is right censoring only. A unit marked failed = false is
  suspended at its listed time: it contributes a reliability factor,
  not a density factor, to the likelihood. There is no left or
  interval censoring in the library, so no fixture uses it.
- Estimator is maximum likelihood (MLE) for every fitting fixture.
- Deterministic samples are built from the fitted distribution's own
  quantile function evaluated at Benard plotting positions
  p_i = (i - 0.3) / (n + 0.4). That construction uses only the
  definition of a quantile sample from known generating parameters;
  the fit under test must then recover those generating parameters
  from the sample alone. The anchor is the generating parameter, not
  the fitted output.

## Family 1: distribution consistency

Source for every case: hand derivation from the distribution
definitions in `src/distributions.rs` module documentation, checked
by two independent paths inside the test: numerical quadrature of
the pdf (composite Simpson rule written in the test) against the
cdf, and the analytic quantile definition against the cdf.

| Case | Distribution and parameters | Points | Expected | Tolerance |
| --- | --- | --- | --- | --- |
| D1 | weibull2, beta = 2.0, eta = 1000 | quantiles at p = 0.25, 0.50, 0.75 | integral of pdf from 0 to t equals cdf(t); cdf(quantile(p)) equals p; reliability equals 1 - cdf | quadrature 2e-3 absolute; quantile inversion 1e-8; reliability 1e-12 |
| D2 | lognormal, mu = 7.0, sigma = 0.5 | same three quantiles | same three identities | same |
| D3 | gamma, alpha = 2.5, beta (scale) = 400 | same three quantiles | same three identities | same |
| D4 | loglogistic, alpha = 1000, beta = 2.0 | same three quantiles | same three identities | same |
| D5 | exponential2, theta = 500, gamma = 100 | same three quantiles | same identities, with the integral starting at the support start gamma = 100 | same |
| D6 | gengamma, a = 1000, d = 2.0, p = 1.5 | same three quantiles | same three identities | same |
| D7 | support check for D1 to D6 | t below the support | pdf = 0 and cdf = 0 below the support. For exponential2 the check point is gamma - 1 = 99. For the other five the check points are t = -10 and the behaviour at 0 stated in the code (pdf 0 at t = 0 for the chosen shapes) | exact 0 |

Parameterization notes:

- gamma uses shape alpha and scale beta, so the mean is
  alpha * beta = 1000.
- gengamma uses the Stacy form in the module documentation:
  scale a, shapes d and p. At p = 1 it reduces to gamma and at
  d = p it reduces to Weibull; those reductions are covered by
  `tests/dist_tests.rs` and are not repeated here.
- exponential2 uses mean theta and location gamma, support
  t >= gamma, reliability exp(-(t - gamma) / theta) above gamma.

## Family 2: fitting with known generating parameters

Source: hand construction. Each sample is a deterministic quantile
sample from the stated generating parameters (see the convention
above), so the anchor is the generating value itself.

| Case | Data | Generating parameters | Expected recovery | Tolerance |
| --- | --- | --- | --- | --- |
| F1 | weibull2 quantile sample, n = 40, complete (all failures) | beta = 2.0, eta = 1000 | fitted beta and eta near the generating values | beta within 25 percent; eta within 15 percent |
| F2 | lognormal quantile sample, n = 40, complete | mu = 7.0, sigma = 0.5 | fitted mu and sigma near the generating values | mu within 0.15; sigma within 25 percent |
| F3 | weibull2 quantile sample, n = 40, largest 25 percent suspended at their sample times (right censoring) | beta = 2.0, eta = 1000 | fitted beta and eta near the generating values under the right-censoring convention above | beta within 35 percent; eta within 25 percent |
| F4 | ties: the sample 100, 100, 200, 200, 300, 300, 500, 500, 800, 800, complete, fitted as weibull2 | no generating value; the anchor is regularity | fit reports success, parameters and log-likelihood are finite and positive where the parameter must be positive | finiteness only |
| F5 | small sample: 120, 450, 900, complete, fitted as weibull2, n = 3 | no generating value; the anchor is regularity | fit returns, parameters finite, no NaN in parameters, log-likelihood, or standard errors that are present | finiteness only |
| F6 | extreme scales: F1 sample multiplied by 1e-9 (times near 1e-6) and by 1e5 (times near 1e8), fitted as weibull2 | beta = 2.0, eta = 1000 times the same scale factor | no NaN; beta near 2.0 at both scales; eta scales with the data | beta within 25 percent; eta within 15 percent of the scaled generating eta |
| F7 | row-order invariance: F1 sample and the same sample reversed and rotated, fitted as weibull2 | anchor is the F1 fit computed in the same test by the deterministic profile path | shuffled fits give the same beta and eta as the ordered fit | 1e-9 relative |
| F8 | unit-rescaling invariance: F1 sample in hours and the same sample times 3600 (seconds), fitted as weibull2 | anchor is the hours fit in the same test plus the definition of a scale parameter | beta unchanged; eta in seconds equals 3600 times eta in hours | 1e-6 relative |

## Family 3: uncertainty spot check (second computation path)

Source: second computation path inside the test.

- Data: the F1 weibull2 quantile sample (n = 40, beta = 2.0,
  eta = 1000 generating values).
- The library fit gives fitted parameters, a log-likelihood, and
  Fisher standard errors in `FitResult.se`.
- The test recomputes the observed information independently: it
  evaluates the library `loglik` function (the likelihood
  definition, not the standard error code) on a finite-difference
  grid around the fitted point, with its own step choice and its
  own 2 by 2 matrix inverse written in the test, and takes
  standard errors as the square roots of the inverse diagonal.
- Expected: the two standard error paths agree.
- Tolerance: 15 percent relative per parameter. The tolerance is
  wide because the two paths use different finite-difference steps
  on purpose; agreement inside it confirms the same Hessian is
  being inverted, and a factor error or a transposed covariance
  would fall far outside it.

## Family 4: fault trees

Covered in `tests/v032_ft_war_tests.rs`, not duplicated here.
That file checks, against hand values and an independent brute
force truth-table enumeration written in that file: A OR A = 0.1,
A AND A = 0.1, (A AND B) OR (A AND C) = 0.044 with a shared A, a
voting gate over a repeated event, rejection of a probability of
1.5 and of NaN with the event named, rejection of conflicting
probabilities for one name, two mixed trees against the truth
table, and the large-tree fallback and limit error. No shape used
in the validation plan is missing there, so this file adds no
fault tree test.

## Family 5: RBD closed forms

Source: hand computation from the series, parallel, and
k-out-of-n definitions, and the binomial sum.

| Case | System | Hand value | Tolerance |
| --- | --- | --- | --- |
| R1 | series of two exponential blocks, theta = 1000 and theta = 2000, at t = 500, through `system::evaluate_rbd` | exp(-500/1000) * exp(-500/2000) = exp(-0.75) | 1e-12 |
| R2 | parallel of the same two blocks at t = 500 | 1 - (1 - exp(-0.5)) * (1 - exp(-0.25)) | 1e-12 |
| R3 | 2-out-of-3 with block reliabilities 0.9, 0.8, 0.7, through `evaluate_rbd` and `kofn_reliability` | Poisson-binomial sum over subsets of size 2 or more, enumerated by hand in the test: the size 2 terms plus the size 3 term | 1e-12 |
| R4 | cold standby, exponential theta = 1000, t = 1000, n = 2 and n = 3, switch reliability 1.0 and 0.95 | Poisson form exp(-lam t) * sum_{i=0}^{n-1} (sw * lam t)^i / i! with lam = 1/1000, so lam t = 1. For n = 2, sw = 1: exp(-1) * 2. This is the same closed form anchored in `tests/standby_tests.rs` (case `closed_form_two_units_is_poisson_sum` and `exponential_cold_standby_matches_closed_forms`); the validation test checks `standby_closed_form_exp` and `system::cold_standby_exponential` against the hand sum written in the test, it does not copy either function's output as the expectation | 1e-12 for the two closed form functions |

## Family 6: availability

Source: hand computation from the single-unit CTMC balance
equations, plus a Monte Carlo agreement check.

| Case | Setup | Hand value | Tolerance |
| --- | --- | --- | --- |
| A1 | one repairable unit, failure rate lam = 0.001 (MTTF 1000), repair rate mu = 0.01 (MTTR 100). CTMC from `system::standby_ctmc` with 1 unit, steady state from `system::ctmc_steady_state`; availability is the up state probability | mu / (lam + mu), which is the same number as MTTF / (MTTF + MTTR) = 1000 / 1100 | 1e-9 |
| A2 | `system::single_block_availability(1000, 100)` | 1000 / 1100 | exact to 1e-12 |
| A3 | Monte Carlo: `system::simulate_availability` on a single block with mtbf = 1000, mttr = 100, horizon 20000, 200 simulations, seed 3, 20 grid points | steady state value from A1, 1000 / 1100 = 0.909090... | 3 Monte Carlo standard errors: the difference must be at most 3 times the `std_error` on the result. See the note below |

Note on A3 and the Monte Carlo standard error:
`AvailabilityResult` in `src/system.rs` carries `std_error`, the
standard error of the per-simulation time-averaged availability
across the seeded runs. The test asserts that `std_error` is
positive and finite and that the grid mean availability differs
from the CTMC steady state by at most 3 standard errors. With
the fixed seed the run is deterministic: mean availability
0.91175, standard error 0.00477, difference 0.00266, inside the
3 standard error bound of 0.0143. The small positive difference
is the start-up transient: every run starts in the up state.

## Family 7: warranty

Covered in `tests/v032_ft_war_tests.rs`, not duplicated here.
That file checks cohort conservation (counted returns plus units
in service equals sales, in total and per cohort), the
future-return exclusion (a return after the cutoff period changes
nothing and is counted in `excluded_future_returns`), the excess
returns error naming the cohort and both counts, the age
convention (a return in the sale period is a failure at age 1,
stated in the result notes as r - s + 1 against censoring at
c - s + 1), and that a return recorded before its sale period
does not enter the fit.

## Family 8: ALT temperature handling (finding recorded)

Source: hand computation with the Arrhenius definition
AF = exp(Ea / k * (1 / T_use - 1 / T_stress)), Ea in eV,
k = 8.617e-5 eV/K (the library constant in `src/alt.rs` is
8.617333262e-5 eV/K; the difference moves the hand value by far
less than the 1 percent tolerance), temperatures in Kelvin inside
the exponential.

What the library actually does, read from `src/alt.rs` at the
time of writing:

- `acceleration_factor_arrhenius(ea_ev, t_use_c, t_stress_c)` takes
  Celsius inputs and converts each to Kelvin by adding 273.15
  before applying the formula. Its doc comment states this. The
  test checks it against the hand value.
- Hand value for the test case: Ea = 0.7 eV, use 25 C (298.15 K),
  stress 85 C (358.15 K). 1/298.15 - 1/358.15 = 0.0005617 per K
  (to four places), Ea / k = 8123.5 K (with k = 8.617e-5), product
  4.562, AF = exp(4.562) = 95.8. The test computes the exponential
  in the test from the stated constants and compares within
  1 percent, and also checks the result is near 95.8 so the formula
  itself is anchored, not just self-consistent.
- The least squares and MLE life-stress fits in the same file
  (`AltFit::mean_log_life`, the design matrix, and the MLE mean)
  use a heuristic instead: a temperature value below 400 is
  treated as Celsius and has 273.15 added; a value of 400 or more
  is treated as already Kelvin. So a Kelvin input below 400 K
  would be shifted a second time, and a Celsius input of 400 or
  more would not be shifted at all. This is a finding, recorded
  here and in the release report. It is not changed by this
  validation work (the library is not part of this change set),
  and no test asserts the heuristic is correct; the fixture only
  documents it so users of the Arrhenius and Eyring fits know to
  pass Celsius values below 400, which is the range the fits were
  built and tested with.
- The two-stress loglinear MLE (`fit_alt_mle_loglinear`) is
  different again: its stress rows are [temperature in Kelvin,
  non-thermal stress] and the Kelvin value is used exactly as
  given, with no conversion. Its doc comment states this. Users
  must not mix the two conventions: Celsius (below 400) for the
  Arrhenius, IPL, and Eyring fits; Kelvin for the loglinear fit
  and for the hand formula inside the exponential.
