//! ReliaKit: reliability engineering toolkit in Rust.
//!
//! A Rust rewrite of the Python ReliaKit suite. Includes life-data
//! analysis (Weibull++ style), limited-data methods (hierarchical
//! shrinkage, shape bias correction, split-conformal bounds),
//! Bayesian MCMC, degradation analysis,
//! accelerated life testing, reliability growth, system reliability
//! with standby redundancy and availability simulation (BlockSim
//! style), fault trees, FMEA worksheets, warranty analysis,
//! parts-count prediction, and two-level factorial DOE.
//!
//! Everything is self-contained: special functions, optimisation, a
//! seedable RNG and an adaptive Metropolis sampler are implemented in
//! this crate. No proprietary handbook data is included anywhere; the
//! prediction module computes only from failure rates the user
//! supplies.

pub mod alt;
pub mod bayes;
pub mod competing;
pub mod datasets;
pub mod degradation;
pub mod distributions;
pub mod doe;
pub mod faulttree;
pub mod flowchart;
pub mod fmea;
pub mod growth;
pub mod mcmc;
pub mod optim;
pub mod prediction;
pub mod rng;
pub mod specfunc;
pub mod standby;
pub mod system;
pub mod warranty;
