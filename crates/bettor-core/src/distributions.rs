//! Monte Carlo sampling for player-prop simulation.
//!
//! Ported from `bettor-calculator-main/src/lib/math/distributions.ts`.
//!
//! # Determinism
//!
//! Every sampler takes an explicit `seed`. This crate never reads entropy —
//! that is the shell's job, so that a simulation is reproducible from the
//! numbers that produced it. The TypeScript called `Math.random()`, which
//! meant identical inputs gave different answers on every click and no
//! simulation in the app could be tested at all.
//!
//! # Divergences from the TypeScript
//!
//! 1. **Poisson is exact at every rate.** The TS switched to a rounded normal
//!    approximation above λ = 30 while its doc comment claimed Ahrens-Dieter
//!    rejection. A normal is symmetric and Poisson is right-skewed, so the
//!    approximation biased the tail — exactly the region a prop line sits in.
//!    `rand_distr` uses a proper method at all rates.
//! 2. **Invalid parameters are errors, not silent zeros.** `generateSamples`
//!    returned an array of zeros for a lognormal with `mu <= 0`, which charts
//!    as a confident spike at zero.
//! 3. **Sample standard deviation, not population.** The TS divided by `n`;
//!    this divides by `n - 1`. At Monte Carlo sample sizes the difference is
//!    invisible, but the estimator is the correct one.

use crate::{MathError, Result};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution as _, Gamma, LogNormal, Poisson, StandardNormal};
use serde::{Deserialize, Serialize};

/// Which family to sample from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Distribution {
    /// Counts with variance equal to the mean. Goals, aces.
    Poisson,
    /// Overdispersed counts — variance a multiple of the mean.
    Nbinom,
    /// Continuous, right-skewed, variance twice the mean.
    Gamma,
    /// Continuous, heavier right tail. Yardage, minutes.
    Lognormal,
}

impl Distribution {
    /// Whether values are whole numbers, which decides how they get binned.
    #[must_use]
    pub const fn is_discrete(self) -> bool {
        matches!(self, Self::Poisson | Self::Nbinom)
    }
}

/// Builds the deterministic generator used by every sampler here.
#[must_use]
pub fn rng_from_seed(seed: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed)
}

fn check_mu(mu: f64) -> Result<()> {
    if !mu.is_finite() || mu <= 0.0 {
        return Err(MathError::DomainError {
            param: "mu",
            constraint: "greater than zero",
            value: mu,
        });
    }
    Ok(())
}

/// Draws `n` samples.
///
/// Parameterisations match the TypeScript so existing presets still mean the
/// same thing:
///
/// - Poisson — `λ = mu`, so variance equals the mean
/// - Negative binomial — mean `mu`, variance `var_multiplier · mu`
/// - Gamma — shape `mu/2`, scale `2`, so variance is `2·mu`
/// - Lognormal — `sdlog = 0.75`, `meanlog = ln(mu) − sdlog²/2`, so the mean is `mu`
///
/// # Errors
///
/// [`MathError::DomainError`] for a non-positive `mu`, a `var_multiplier` at or
/// below 1.0 for the negative binomial, or parameters a sampler rejects.
pub fn generate_samples(
    n: usize,
    distribution: Distribution,
    mu: f64,
    var_multiplier: f64,
    seed: u64,
) -> Result<Vec<f64>> {
    check_mu(mu)?;
    let mut rng = rng_from_seed(seed);

    let domain = |param: &'static str, constraint: &'static str, value: f64| MathError::DomainError {
        param,
        constraint,
        value,
    };

    match distribution {
        Distribution::Poisson => {
            let dist = Poisson::new(mu).map_err(|_| domain("mu", "a valid Poisson rate", mu))?;
            Ok((0..n).map(|_| dist.sample(&mut rng)).collect())
        }
        Distribution::Nbinom => {
            // NB(r, p) has mean r(1−p)/p and variance mean/p. For a variance of
            // k·mean: p = 1/k and r = mu/(k−1).
            if !var_multiplier.is_finite() || var_multiplier <= 1.0 {
                return Err(domain(
                    "var_multiplier",
                    "greater than 1 (at 1 the negative binomial degenerates to Poisson)",
                    var_multiplier,
                ));
            }
            let k = var_multiplier;
            let r = mu / (k - 1.0);
            let gamma_scale = k - 1.0;
            let gamma = Gamma::new(r, gamma_scale)
                .map_err(|_| domain("mu/var_multiplier", "valid gamma parameters", r))?;
            (0..n)
                .map(|_| {
                    let lambda: f64 = gamma.sample(&mut rng);
                    if lambda <= 0.0 {
                        return Ok(0.0);
                    }
                    Poisson::new(lambda)
                        .map(|p| p.sample(&mut rng))
                        .map_err(|_| domain("lambda", "a valid Poisson rate", lambda))
                })
                .collect()
        }
        Distribution::Gamma => {
            let gamma = Gamma::new(mu / 2.0, 2.0)
                .map_err(|_| domain("mu", "valid gamma parameters", mu))?;
            Ok((0..n).map(|_| gamma.sample(&mut rng)).collect())
        }
        Distribution::Lognormal => {
            let sdlog = 0.75;
            let meanlog = mu.ln() - sdlog * sdlog / 2.0;
            let dist = LogNormal::new(meanlog, sdlog)
                .map_err(|_| domain("mu", "valid lognormal parameters", mu))?;
            Ok((0..n).map(|_| dist.sample(&mut rng)).collect())
        }
    }
}

/// Draws a single standard normal variate.
pub fn std_normal(rng: &mut ChaCha8Rng) -> f64 {
    rng.sample(StandardNormal)
}

/// One bar of a histogram.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct HistogramBin {
    /// Left edge, inclusive.
    pub range_start: f64,
    /// Right edge, exclusive (except in the final bin).
    pub range_end: f64,
    /// How many samples landed here.
    pub count: usize,
    /// [`Self::count`] as a fraction of all samples.
    pub freq: f64,
}

/// Bins samples for display.
///
/// Discrete families get one bin per integer; continuous families get
/// `bin_count` equal-width bins, defaulting to Sturges' rule with a floor of 20.
#[must_use]
pub fn histogram(
    samples: &[f64],
    distribution: Distribution,
    bin_count: Option<usize>,
) -> Vec<HistogramBin> {
    if samples.is_empty() {
        return Vec::new();
    }
    #[allow(clippy::cast_precision_loss, reason = "sample count for a frequency")]
    let total = samples.len() as f64;

    if distribution.is_discrete() {
        let max = samples
            .iter()
            .fold(0.0_f64, |m, s| m.max(s.round()))
            .max(0.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "max is non-negative and bounded by realistic prop values"
        )]
        let max_idx = max as usize;
        let mut counts = vec![0_usize; max_idx + 1];
        for s in samples {
            let v = s.round();
            if v >= 0.0 {
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "guarded non-negative and bounded by max above"
                )]
                let idx = v as usize;
                if let Some(slot) = counts.get_mut(idx) {
                    *slot += 1;
                }
            }
        }
        return counts
            .into_iter()
            .enumerate()
            .map(|(i, count)| {
                #[allow(clippy::cast_precision_loss, reason = "bin index is small")]
                let edge = i as f64;
                #[allow(clippy::cast_precision_loss, reason = "count for a frequency")]
                let freq = count as f64 / total;
                HistogramBin {
                    range_start: edge,
                    range_end: edge,
                    count,
                    freq,
                }
            })
            .collect();
    }

    let min = samples.iter().fold(f64::INFINITY, |m, s| m.min(*s));
    let max = samples.iter().fold(f64::NEG_INFINITY, |m, s| m.max(*s));
    let n_bins = bin_count.unwrap_or_else(|| {
        #[allow(clippy::cast_precision_loss, reason = "sample count for Sturges' rule")]
        let sturges = 1.0 + 3.322 * (samples.len() as f64).log10();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "sturges is small and positive"
        )]
        let bins = sturges.ceil() as usize;
        bins.max(20)
    });

    let width = (max - min) / {
        #[allow(clippy::cast_precision_loss, reason = "bin count is small")]
        let w = n_bins as f64;
        w
    };
    if width.is_nan() || width <= 0.0 {
        return vec![HistogramBin {
            range_start: min,
            range_end: min,
            count: samples.len(),
            freq: 1.0,
        }];
    }

    let mut counts = vec![0_usize; n_bins];
    for s in samples {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped into range immediately below"
        )]
        let raw = ((s - min) / width).floor().max(0.0) as usize;
        let idx = raw.min(n_bins - 1);
        if let Some(slot) = counts.get_mut(idx) {
            *slot += 1;
        }
    }

    counts
        .into_iter()
        .enumerate()
        .map(|(i, count)| {
            #[allow(clippy::cast_precision_loss, reason = "bin index is small")]
            let i_f = i as f64;
            #[allow(clippy::cast_precision_loss, reason = "count for a frequency")]
            let freq = count as f64 / total;
            HistogramBin {
                range_start: min + width * i_f,
                range_end: min + width * (i_f + 1.0),
                count,
                freq,
            }
        })
        .collect()
}

/// Probability the prop goes over `line`, estimated from samples.
///
/// Samples landing exactly on an integer line split evenly between over and
/// under, which is how a push settles.
#[must_use]
pub fn over_prob(samples: &[f64], line: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut above = 0_usize;
    let mut equal = 0_usize;
    for s in samples {
        if *s > line {
            above += 1;
        } else if (*s - line).abs() < f64::EPSILON {
            equal += 1;
        }
    }
    #[allow(clippy::cast_precision_loss, reason = "counts for a proportion")]
    let numerator = above as f64 + equal as f64 / 2.0;
    #[allow(clippy::cast_precision_loss, reason = "sample count")]
    let denominator = samples.len() as f64;
    numerator / denominator
}

/// Mean and standard deviation of a sample.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    /// Sample mean.
    pub mean: f64,
    /// Sample standard deviation, using the `n − 1` denominator.
    pub std_dev: f64,
}

/// Computes mean and standard deviation in one pass (Welford).
#[must_use]
pub fn stats(samples: &[f64]) -> Stats {
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;
    let mut n = 0_u64;
    for s in samples {
        n += 1;
        #[allow(clippy::cast_precision_loss, reason = "running count")]
        let count = n as f64;
        let delta = s - mean;
        mean += delta / count;
        m2 += delta * (s - mean);
    }
    let std_dev = if n > 1 {
        #[allow(clippy::cast_precision_loss, reason = "running count")]
        let denom = (n - 1) as f64;
        (m2 / denom).sqrt()
    } else {
        0.0
    };
    Stats { mean, std_dev }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    reason = "test code"
)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    const N: usize = 200_000;

    #[test]
    fn the_same_seed_gives_the_same_samples() {
        // Impossible in the TypeScript, which is why none of this was tested.
        let a = generate_samples(1_000, Distribution::Poisson, 8.0, 2.0, 42).unwrap();
        let b = generate_samples(1_000, Distribution::Poisson, 8.0, 2.0, 42).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_give_different_samples() {
        let a = generate_samples(1_000, Distribution::Poisson, 8.0, 2.0, 1).unwrap();
        let b = generate_samples(1_000, Distribution::Poisson, 8.0, 2.0, 2).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn poisson_has_variance_equal_to_its_mean() {
        let s = generate_samples(N, Distribution::Poisson, 8.0, 2.0, 7).unwrap();
        let st = stats(&s);
        assert_relative_eq!(st.mean, 8.0, epsilon = 0.05);
        assert_relative_eq!(st.std_dev * st.std_dev, 8.0, epsilon = 0.1);
    }

    #[test]
    fn poisson_stays_skewed_above_lambda_thirty() {
        // The TS switched to a rounded normal here, which is symmetric. A real
        // Poisson keeps a positive skew of 1/sqrt(lambda).
        let s = generate_samples(N, Distribution::Poisson, 50.0, 2.0, 11).unwrap();
        let st = stats(&s);
        let skew = s
            .iter()
            .map(|x| ((x - st.mean) / st.std_dev).powi(3))
            .sum::<f64>()
            / N as f64;
        assert_relative_eq!(skew, 1.0 / 50.0_f64.sqrt(), epsilon = 0.03);
        assert_relative_eq!(st.mean, 50.0, epsilon = 0.1);
    }

    #[test]
    fn negative_binomial_hits_its_requested_overdispersion() {
        for k in [1.5, 2.0, 3.0] {
            let s = generate_samples(N, Distribution::Nbinom, 10.0, k, 13).unwrap();
            let st = stats(&s);
            assert_relative_eq!(st.mean, 10.0, epsilon = 0.1);
            assert_relative_eq!(st.std_dev * st.std_dev, k * 10.0, epsilon = k * 0.5);
        }
    }

    #[test]
    fn gamma_has_variance_twice_its_mean() {
        let s = generate_samples(N, Distribution::Gamma, 20.0, 2.0, 17).unwrap();
        let st = stats(&s);
        assert_relative_eq!(st.mean, 20.0, epsilon = 0.1);
        assert_relative_eq!(st.std_dev * st.std_dev, 40.0, epsilon = 1.0);
    }

    #[test]
    fn lognormal_is_centered_on_mu() {
        let s = generate_samples(N, Distribution::Lognormal, 65.0, 2.0, 19).unwrap();
        assert_relative_eq!(stats(&s).mean, 65.0, epsilon = 1.0);
    }

    #[test]
    fn invalid_parameters_error_instead_of_returning_zeros() {
        // TS returned an array of zeros for this, which charts as certainty.
        assert!(generate_samples(10, Distribution::Lognormal, 0.0, 2.0, 1).is_err());
        assert!(generate_samples(10, Distribution::Poisson, -5.0, 2.0, 1).is_err());
        assert!(generate_samples(10, Distribution::Nbinom, 10.0, 1.0, 1).is_err());
    }

    #[test]
    fn histogram_frequencies_sum_to_one() {
        let s = generate_samples(10_000, Distribution::Poisson, 6.0, 2.0, 23).unwrap();
        let bins = histogram(&s, Distribution::Poisson, None);
        let total: f64 = bins.iter().map(|b| b.freq).sum();
        assert_relative_eq!(total, 1.0, epsilon = 1e-9);
        assert_eq!(bins.iter().map(|b| b.count).sum::<usize>(), 10_000);
    }

    #[test]
    fn continuous_histogram_covers_every_sample() {
        let s = generate_samples(10_000, Distribution::Gamma, 20.0, 2.0, 29).unwrap();
        let bins = histogram(&s, Distribution::Gamma, Some(30));
        assert_eq!(bins.len(), 30);
        assert_eq!(bins.iter().map(|b| b.count).sum::<usize>(), 10_000);
    }

    #[test]
    fn a_half_point_line_has_no_pushes() {
        let s = vec![1.0, 2.0, 3.0, 4.0];
        assert_relative_eq!(over_prob(&s, 2.5), 0.5, epsilon = 1e-12);
    }

    #[test]
    fn an_integer_line_splits_pushes_evenly() {
        // Two under, one push, one over: the push splits, giving 0.375.
        let s = vec![1.0, 2.0, 3.0, 4.0];
        assert_relative_eq!(over_prob(&s, 3.0), 0.375, epsilon = 1e-12);
    }

    #[test]
    fn stats_match_a_hand_computed_sample() {
        let s = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let st = stats(&s);
        assert_relative_eq!(st.mean, 5.0, epsilon = 1e-12);
        // Sample SD (n-1) is sqrt(32/7); the TS reported the population 2.0.
        assert_relative_eq!(st.std_dev, (32.0_f64 / 7.0).sqrt(), epsilon = 1e-12);
    }

    #[test]
    fn empty_input_is_handled() {
        assert!(histogram(&[], Distribution::Poisson, None).is_empty());
        assert_relative_eq!(over_prob(&[], 2.5), 0.0, epsilon = 1e-12);
        assert_relative_eq!(stats(&[]).mean, 0.0, epsilon = 1e-12);
    }
}
