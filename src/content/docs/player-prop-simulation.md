---
title: "Player Prop Simulation: Monte Carlo Methods for Sports Betting"
date: "2025-11-15"
excerpt: "How Monte Carlo simulation models player prop outcomes using sport-specific distributions — Poisson for counts, lognormal for yardage, and more."
tags: ["monte-carlo", "props", "simulation", "distributions"]
---

## Why Simulate Props?

Player prop markets are among the least efficient in sports betting. Books set lines based on projections, but the underlying distributions are complex — a quarterback's passing yards do not follow a simple bell curve, and a pitcher's strikeout count behaves differently from a running back's rushing attempts.

Monte Carlo simulation cuts through this complexity by generating tens of thousands of simulated outcomes and letting the distribution emerge from the data.

## How Monte Carlo Works

The process is straightforward:

1. Choose a probability distribution that fits the stat (more on this below)
2. Set the parameters (mean, variance) based on your projection
3. Simulate 100,000+ random outcomes from that distribution
4. Count what fraction of outcomes exceed the book's line
5. Convert that fraction to fair odds

The law of large numbers guarantees that with enough simulations, the estimated probability converges to the true probability. At 100,000 trials, the margin of error is negligible.

## Choosing the Right Distribution

Different stats call for different distributions:

**Poisson** — Best for discrete counting stats with low-to-moderate averages: strikeouts, assists, goals, three-pointers made. The Poisson distribution models the count of independent events occurring at a constant rate.

$$
P(X = k) = \frac{\lambda^k e^{-\lambda}}{k!}
$$

A pitcher projected for 6.5 strikeouts has a specific probability of recording exactly 5, 6, 7, or any other count — and Poisson captures this naturally.

**Lognormal** — Best for continuous, right-skewed stats: passing yards, rushing yards, receiving yards. These stats cannot be negative, have a floor near zero, and occasionally produce extreme high values. The lognormal distribution handles all three properties.

$$
\ln(X) \sim \mathcal{N}(\mu, \sigma^2)
$$

**Negative Binomial** — Best for overdispersed count data where the variance exceeds the mean. Some player stats show more variability than Poisson allows — games where a player goes off for 40 points or is held scoreless both happen more often than Poisson predicts.

## From Simulations to Fair Odds

After running 100,000 simulations with a projected mean of 22.5 points for a player, suppose 54,200 outcomes are over 21.5 (the book's line). The fair probability of the over is:

$$
P(\text{over}) = \frac{54200}{100000} = 54.2\%
$$

Converting to American odds: the fair over is approximately -118. If the book offers the over at -110, that is a +EV opportunity.

## Practical Tips

- Your projection of the mean matters more than the distribution choice — garbage in, garbage out
- Poisson is the safest default for counting stats; lognormal for yardage
- Compare simulated fair odds to market prices — the gap is your estimated edge
- The Player Prop Simulator runs 100,000 simulations with sport-specific distributions and shows you the fair odds for both over and under
