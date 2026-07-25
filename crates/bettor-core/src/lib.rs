//! Sports betting math.
//!
//! This crate is a pure library: no io, no global state, no Tauri. Every public
//! function is deterministic given its inputs — simulations take an explicit
//! seed rather than reaching for a thread-local RNG, so a result can always be
//! reproduced from the values that produced it.
//!
//! The reference implementation being ported is the TypeScript in
//! `bettor-calculator-main/src/lib/math/`. Where behavior here intentionally
//! diverges from that source, the divergence is documented at the definition.

pub mod arbitrage;
pub mod bayesian;
pub mod clv;
pub mod correlation;
pub mod devig;
pub mod distributions;
pub mod hold;
pub mod line;
pub mod match_model;
pub mod middle;
pub mod odds;
pub mod parlay;
pub mod probability;
pub mod regression;
pub mod risk_of_ruin;
pub mod teaser;
pub mod wager;

/// Serialization for RNG seeds.
///
/// A seed is a `u64`, but JSON numbers are IEEE doubles: anything above 2^53
/// loses precision in transit, which would quietly break the promise that a
/// reported result can be reproduced from its seed. Seeds therefore cross the
/// wire as decimal strings. They are opaque tokens, not quantities — nothing
/// downstream does arithmetic on them.
pub mod seed_repr {
    use serde::Serializer;

    /// Serializes a seed as a decimal string.
    ///
    /// # Errors
    ///
    /// Propagates the serializer's own failure.
    pub fn serialize<S: Serializer>(seed: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&seed.to_string())
    }

    /// Serializes an optional seed as an optional decimal string.
    ///
    /// # Errors
    ///
    /// Propagates the serializer's own failure.
    pub fn serialize_option<S: Serializer>(seed: &Option<u64>, s: S) -> Result<S::Ok, S::Error> {
        match seed {
            Some(v) => s.serialize_some(&v.to_string()),
            None => s.serialize_none(),
        }
    }
}

/// Version of the math engine, surfaced in the UI so a reported result can
/// always be traced back to the code that produced it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Errors produced by invalid input to a math routine.
///
/// The TypeScript returns `null` on bad input, which collapses "you typed a
/// letter", "that probability is out of range", and "this solver did not
/// converge" into one indistinguishable value. Each gets its own variant here
/// so the UI can say something useful.
#[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum MathError {
    /// A string input could not be parsed in the expected odds format.
    #[error("could not parse {value:?} as {format} odds")]
    ParseOdds {
        /// The offending input, echoed back so the UI can highlight it.
        value: String,
        /// The format that was expected: `"american"`, `"decimal"`, `"fractional"`.
        format: &'static str,
    },

    /// A probability fell outside the range a routine accepts.
    #[error("probability {value} is out of range: {reason}")]
    ProbabilityOutOfRange {
        /// The probability that was rejected.
        value: f64,
        /// Why it was rejected, e.g. `"must be strictly between 0 and 1"`.
        reason: &'static str,
    },

    /// A numeric argument was outside the domain the routine accepts.
    #[error("{param} must be {constraint}, got {value}")]
    DomainError {
        /// Name of the parameter, matching the public API's argument name.
        param: &'static str,
        /// The constraint that was violated, e.g. `"greater than zero"`.
        constraint: &'static str,
        /// The value supplied.
        value: f64,
    },

    /// A collection argument had the wrong shape (too few legs, mismatched lengths).
    #[error("expected {expected} {what}, got {got}")]
    ShapeError {
        /// What was being counted, e.g. `"parlay legs"`.
        what: &'static str,
        /// The requirement, e.g. `"at least 2"`.
        expected: &'static str,
        /// The count actually supplied.
        got: usize,
    },

    /// An iterative solver ran out of iterations without meeting its tolerance.
    ///
    /// Reported rather than swallowed: a devig method that silently returns its
    /// last un-converged iterate is worse than one that admits it failed.
    #[error("{solver} failed to converge within {iterations} iterations (residual {residual:e})")]
    NoConvergence {
        /// Which solver gave up, e.g. `"shin"`.
        solver: &'static str,
        /// The iteration cap it hit.
        iterations: u32,
        /// How far from tolerance it still was when it stopped.
        residual: f64,
    },
}

/// Convenience alias for fallible math.
pub type Result<T> = core::result::Result<T, MathError>;
