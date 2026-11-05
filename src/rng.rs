//! Deterministic seeded RNG (xoshiro256** with splitmix64 seeding) and
//! distribution samplers built on inverse CDF and Box-Muller. Tests and
//! benchmarks use fixed seeds only; nothing here is unseeded.

pub struct Rng64 {
    s: [u64; 4],
    spare_normal: Option<f64>,
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

impl Rng64 {
    pub fn new(seed: u64) -> Self {
        let mut st = seed ^ 0xA0761D6478BD642F;
        let s = [
            splitmix64(&mut st),
            splitmix64(&mut st),
            splitmix64(&mut st),
            splitmix64(&mut st),
        ];
        Rng64 { s, spare_normal: None }
    }

    /// Derive an independent stream (used for MCMC chains).
    pub fn fork(&mut self) -> Rng64 {
        Rng64::new(self.next_u64())
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let result = (self.s[1].wrapping_mul(5)).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform on [0, 1).
    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform on (0, 1], safe for log transforms.
    pub fn open01(&mut self) -> f64 {
        1.0 - self.next_f64()
    }

    pub fn normal(&mut self) -> f64 {
        if let Some(z) = self.spare_normal.take() {
            return z;
        }
        let u1 = self.open01();
        let u2 = self.next_f64();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        self.spare_normal = Some(r * theta.sin());
        r * theta.cos()
    }

    pub fn exponential(&mut self, mean: f64) -> f64 {
        -mean * self.open01().ln()
    }

    pub fn weibull(&mut self, beta: f64, eta: f64) -> f64 {
        eta * (-self.open01().ln()).powf(1.0 / beta)
    }

    pub fn lognormal(&mut self, mu: f64, sigma: f64) -> f64 {
        (mu + sigma * self.normal()).exp()
    }

    pub fn gamma(&mut self, shape: f64, scale: f64) -> f64 {
        // Marsaglia and Tsang.
        if shape < 1.0 {
            let u = self.open01();
            return self.gamma(shape + 1.0, scale) * u.powf(1.0 / shape);
        }
        let d = shape - 1.0 / 3.0;
        let c = 1.0 / (9.0 * shape).sqrt();
        loop {
            let mut x = self.normal();
            let mut v = 1.0 + c * x;
            if v <= 0.0 {
                continue;
            }
            v = v * v * v;
            let u = self.open01();
            x = x * x;
            if u < 1.0 - 0.0331 * x * x || u.ln() < 0.5 * x + d * (1.0 - v + v.ln()) {
                return d * v * scale;
            }
        }
    }
}
