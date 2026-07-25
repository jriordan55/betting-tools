---
title: "Better Line: Using CDF Analysis to Compare Betting Lines"
date: "2025-07-15"
excerpt: "How to use cumulative distribution functions to determine which of two lines has less embedded vig — and find the sharper price."
tags: ["cdf", "line-comparison", "vig", "modeling"]
---

## The Problem

Two books offer different lines on the same spread market. Book A has Team X -3.5 at -110. Book B has Team X -3 at -120. Which is the better deal?

It is not obvious. One offers a better number, the other offers better juice. You cannot compare them by just looking at the odds — you need to model the underlying outcome distribution and ask: what is the fair probability of each line covering?

## CDF-Based Line Comparison

The cumulative distribution function (CDF) gives the probability that a random variable is less than or equal to a given value. For spreads, we model the scoring margin as a continuous distribution (typically normal) and use the CDF to price any line:

$$
P(\text{cover}) = 1 - \Phi\left(\frac{s - \mu}{\sigma}\right)
$$

where $s$ is the spread, $\mu$ is the true expected margin, and $\sigma$ is the standard deviation of outcomes.

## Backing Out the True Center

Each book's line and odds imply a true center $\mu$. By inverting the CDF, we can extract the implied center from each book's offering:

$$
\mu = s - \sigma \cdot \Phi^{-1}(1 - p_{\text{implied}})
$$

If Book A and Book B imply different centers, the one closer to the consensus or the one with less vig embedded is the sharper price.

## Comparing the Lines

Once you have the fair probability of each line covering (derived from the CDF model), compare it to the implied probability from the odds. The line with the bigger gap between fair probability and implied probability has more embedded vig — and the other line is the better bet.

For example, if the CDF model says -3.5 has a 48.2% fair probability of covering, but Book A implies 52.4% (at -110), that is 4.2% of embedded vig. If the model says -3 has a 51.5% fair probability, but Book B implies 54.5% (at -120), that is 3.0% of embedded vig. Book B is the better deal despite the higher juice.

## When Does This Matter?

CDF comparison is most valuable when:

- **Lines differ by a key number** (3, 7 in football) where small moves in the spread create large jumps in cover probability
- **Juice structures differ significantly** — one book offers a better number with worse juice
- **You are choosing between two similar-looking options** and need a principled tiebreaker

## Practical Tips

- The standard deviation $\sigma$ varies by sport: roughly 13.5 for NFL, 10-12 for NBA, 1.2 for soccer
- Key number effects (clustering around 3 and 7 in football) mean half-point differences can be worth more than they appear
- The Better Line calculator automates this comparison — input two lines and it tells you which has less embedded vig
