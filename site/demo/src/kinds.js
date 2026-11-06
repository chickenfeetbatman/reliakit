// Shared kind metadata for the 18 item kinds.
// Order matches KIND_OPTIONS in screens.js (and KIND_DEFS in the
// engine), so kindIndex(key) is the index collectFields sends as
// app.kind_index when dispatching "add-item".
export const FAMILIES = ["Life data analysis", "System modeling", "Risk and maintenance", "Prediction and design"];

export const KIND_META = [
  { key: "life", label: "Life Data", icon: "life", family: "Life data analysis", desc: "Fit distributions to failure and suspension times and read reliability at any time." },
  { key: "limited", label: "Limited Data", icon: "limited", family: "Life data analysis", desc: "Get honest answers from very small samples: bias correction, shrinkage, and bounds." },
  { key: "mcmc", label: "MCMC", icon: "mcmc", family: "Life data analysis", desc: "Bayesian fitting with sampled posteriors and convergence diagnostics." },
  { key: "degradation", label: "Degradation", icon: "degradation", family: "Life data analysis", desc: "Project measured wear paths to a failure threshold." },
  { key: "alt", label: "ALT", icon: "alt", family: "Life data analysis", desc: "Use accelerated test data to predict life at normal stress." },
  { key: "growth", label: "Growth", icon: "growth", family: "Life data analysis", desc: "Track reliability growth across a test-fix-test program." },
  { key: "competing", label: "Competing Risks", icon: "competing", family: "Life data analysis", desc: "Separate several failure causes and their cumulative incidence." },
  { key: "rbd", label: "RBD", icon: "rbd", family: "System modeling", desc: "Build a block diagram of a system and compute its reliability and availability." },
  { key: "faulttree", label: "Fault Tree", icon: "faulttree", family: "System modeling", desc: "Model failure logic with gates and find the minimal cut sets." },
  { key: "fmea", label: "FMEA", icon: "fmea", family: "Risk and maintenance", desc: "Worksheet for failure modes, RPN, FMECA criticality, and DVP&R." },
  { key: "warranty", label: "Warranty", icon: "warranty", family: "Risk and maintenance", desc: "Forecast returns from sales and claims with a Nevada table." },
  { key: "prediction", label: "Prediction", icon: "prediction", family: "Prediction and design", desc: "Predict failure rates from parts data and standards libraries." },
  { key: "doe", label: "DOE", icon: "doe", family: "Prediction and design", desc: "Design two-level experiments and read factor effects." },
  { key: "flowchart", label: "Flowchart", icon: "flowchart", family: "System modeling", desc: "Simulate a process flow with Monte Carlo runs." },
  { key: "markov", label: "Markov", icon: "markov", family: "System modeling", desc: "Model repairable systems as state diagrams with transition rates." },
  { key: "qcp", label: "QCP", icon: "qcp", family: "Prediction and design", desc: "Quick reliability math: reliability, hazard, mean life, and Bx life for any distribution." },
  { key: "rcm", label: "RCM", icon: "rcm", family: "Risk and maintenance", desc: "Choose maintenance tasks from consequences and failure patterns." },
  { key: "fracas", label: "FRACAS", icon: "fracas", family: "Risk and maintenance", desc: "Log failures, track corrective actions, and watch MTBF." },
];

export function kindIndex(key) {
  return KIND_META.findIndex((k) => k.key === key);
}

export function kindByKey(key) {
  return KIND_META.find((k) => k.key === key) || null;
}
