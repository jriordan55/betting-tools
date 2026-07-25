---
title: "Regression to the Mean: Why Small Samples Lie in Sports Betting"
date: "2025-10-15"
excerpt: "How Bayesian shrinkage corrects small-sample player stats toward true talent — and why it matters for props and projections."
tags: ["regression", "bayesian", "props", "small-sample"]
---

## The Small-Sample Trap

A baseball player is hitting .400 after 50 at-bats. A quarterback has a 75% completion rate through 3 games. An NBA player is shooting 45% from three on 40 attempts. Are these real talent levels?

Almost certainly not. Small samples are dominated by noise. **Regression to the mean** is the statistical correction that pulls extreme observations back toward the population average — and it is one of the most important concepts in sports betting.

## The Bayesian Shrinkage Formula

The regressed estimate blends the observed stat with a population baseline:

$$
\hat{p} = p_{\text{baseline}} + (p_{\text{observed}} - p_{\text{baseline}}) \cdot \frac{n}{n + k}
$$

where:

- $p_{\text{observed}}$ is the raw stat (e.g., .400 batting average)
- $p_{\text{baseline}}$ is the population average (e.g., .250 for MLB batting)
- $n$ is the sample size (e.g., 50 at-bats)
- $k$ is the **regression constant** — the sample size where observed data gets 50% weight

The weight on observed data is $w = n / (n + k)$. When $n = k$, the weight is exactly 0.5. When $n \ll k$, the estimate stays close to baseline. When $n \gg k$, the estimate trusts the observed data.

## Signal vs. Noise

The regression constant $k$ captures how noisy a stat is. Higher $k$ means more noise:

| Stat | Approximate $k$ | Interpretation |
|------|-----------------|----------------|
| MLB batting average | ~250 ABs | Need 250 ABs for 50/50 signal-to-noise |
| NFL completion % | ~250 attempts | Very noisy in small samples |
| NBA 3PT % | ~500 attempts | Extremely noisy |
| Soccer shot conversion | ~150 shots | Moderate noise |

With $k = 250$ and only 50 observations, you place just $50/(50+250) = 16.7\%$ weight on what you have seen. The other 83.3% comes from the baseline.

## Worked Example

That .400 hitter after 50 at-bats, with a league baseline of .250 and $k = 250$:

$$
\hat{p} = .250 + (.400 - .250) \times \frac{50}{50 + 250} = .250 + .150 \times 0.167 = .275
$$

The regressed estimate is .275 — far from the raw .400. After 200 at-bats of the same performance:

$$
\hat{p} = .250 + .150 \times \frac{200}{200 + 250} = .250 + .150 \times 0.444 = .317
$$

The data starts to speak louder, but still pulls toward the mean.

## Why This Matters for Betting

Sportsbooks set player prop lines. When a player is on a hot streak, the public hammers the over and the line inflates. But if the streak is driven by small-sample noise, the regressed projection is much lower than the raw stat suggests.

This creates value on the under. Conversely, a slumping player with a long track record may have an artificially depressed line — creating value on the over.

## Practical Tips

- Always regress before using a raw stat in a projection model
- The less stable the stat, the more you should trust the baseline over recent performance
- Track how the regressed estimate converges as the season progresses — this is what the Regression to the Mean calculator visualizes
- For props, compare the regressed projection to the book's line to identify mispriced opportunities
