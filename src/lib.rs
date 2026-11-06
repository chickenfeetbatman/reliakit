//! ReliaKit: reliability engineering in Rust.
//!
//! A rewrite of the Python ReliaKit suite. Life-data analysis
//! (Weibull++ style), limited-data methods (hierarchical shrinkage,
//! shape bias correction, split-conformal bounds), Bayesian MCMC,
//! degradation, accelerated life testing, reliability growth,
//! system reliability with standby redundancy and availability
//! simulation (BlockSim style), fault trees, FMEA, warranty,
//! parts-count prediction, and two-level factorial DOE.
//!
//! The crate has its own special functions, optimizer, seeded RNG,
//! and adaptive Metropolis sampler. No proprietary handbook data is
//! bundled; the prediction module computes from failure rates the
//! user supplies.

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
pub mod fracas;
pub mod growth;
pub mod importance;
pub mod markov;
pub mod mcmc;
pub mod optim;
pub mod prediction;
pub mod quickcalc;
pub mod rcm;
pub mod rng;
pub mod specfunc;
pub mod standby;
pub mod standards217;
pub mod system;
pub mod warranty;
pub mod wizard;
