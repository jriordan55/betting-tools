---
title: "5 Devig Methods Compared: Which One Should You Use?"
date: "2025-02-01"
excerpt: "A deep dive into Equal Margin, Proportional, Shin, Odds Ratio, and Logarithmic devig methods — when each works best."
tags: ["devig", "probability", "vig"]
---

## The Problem

Bookmakers build a margin (vig) into their odds. A "fair" -110/-110 market has implied probabilities summing to 104.76% instead of 100%. To find the true probability, you need to remove the vig — but *how* you remove it matters.

There are five common methods, each with different assumptions about how bookmakers distribute the margin.

## The Five Methods

### 1. Equal Margin (EM)

The simplest approach: subtract an equal share of the margin from each outcome.

$$
p_{\text{fair},i} = p_{\text{implied},i} - \frac{\text{margin}}{n}
$$

**Assumption:** The bookmaker adds the same absolute margin to each outcome. This rarely holds — favorites and longshots usually get different treatment.

### 2. Margin Proportional to Odds (MPTO)

The most common method: divide each implied probability by the total.

$$
p_{\text{fair},i} = \frac{p_{\text{implied},i}}{\sum_j p_{\text{implied},j}}
$$

**Assumption:** The margin is proportional to each outcome's probability. This is the default "normalize" approach and works well for most markets.

### 3. Shin Method

Based on Shin's (1993) insider trading model. The idea is that some fraction $z$ of bettors are "insiders" who know the outcome.

For each outcome, the fair probability satisfies:

$$
p_{\text{fair},i} = \frac{\sqrt{z^2 + 4(1-z) \cdot q_i / S} - z}{2(1-z)}
$$

where $q_i$ is the implied probability, $S$ is the sum of implied probabilities, and $z$ is found via bisection such that fair probabilities sum to 1.

**Assumption:** Part of the margin comes from the book protecting itself against informed bettors. The Shin method typically gives fair odds that are slightly more extreme than MPTO — favorites become slightly bigger favorites, and longshots become slightly bigger longshots.

### 4. Odds Ratio (Power Method)

Find an exponent $c$ such that $\sum_i p_i^c = 1$:

$$
p_{\text{fair},i} = p_{\text{implied},i}^{\ c}
$$

Since $\sum p_i > 1$ at $c = 1$, we need $c > 1$. This is solved via bisection.

**Assumption:** Margin is applied through a power transformation. This method tends to produce results between MPTO and Shin.

### 5. Logarithmic

Adjust in log-odds space by subtracting a constant $k$:

$$
p_{\text{fair},i} = \text{logistic}\!\left(\log\frac{p_i}{1-p_i} - k\right)
$$

where $k$ is found such that the fair probabilities sum to 1.

**Assumption:** The margin is additive in log-odds space. This preserves the relative log-odds ratios between outcomes.

## Shin and Equal Margin Are the Same Thing on a Two-Way Market

Worth knowing before you read the table below and see two columns agreeing.

On a market with exactly two outcomes, Shin returns *precisely* the equal-margin
probabilities — not approximately, and not for some particular market. Invert
Shin's formula to get the vigged price back:

$$
q_i = \sqrt{S \cdot \pi_i \left( z + (1-z)\pi_i \right)}
$$

With two outcomes, write $\pi_1 = p$ and $\pi_2 = 1 - p$. The insider fraction
$z$ then cancels out of the difference:

$$
q_1^2 - q_2^2 = S\left[ z(2p-1) + (1-z)\left(p^2 - (1-p)^2\right) \right] = S(2p-1)
$$

Together with $q_1 + q_2 = S$ that forces $q_1 - q_2 = \pi_1 - \pi_2$, which is
exactly the equal-margin adjustment $\pi_i = q_i - (S-1)/2$.

So on a two-way market you have five methods and four distinct answers. Shin
only says something of its own once there are three or more outcomes — a
soccer 1X2 market, or a three-way prop.

## When Do They Differ?

For balanced markets (e.g., -110/-110), all five methods produce identical
results — there is no margin to distribute. Differences emerge in
**unbalanced markets**. Each cell below is the fair probability of the
**first-listed** outcome:

| Market | Book total | EM | MPTO | Shin | OR | LOG |
|--------|-----------|-----|------|------|-----|-----|
| -300/+250 | 103.57% | 73.21% | 72.41% | 73.21% | 73.63% | 73.25% |
| -110/-110 | 104.76% | 50.00% | 50.00% | 50.00% | 50.00% | 50.00% |
| +500/-700 | 104.17% | 14.58% | 16.00% | 14.58% | 13.75% | 14.46% |

Two things stand out. Shin matches EM exactly, for the reason above. And the
spread between methods is **not** small once the market is lopsided: on the
+500 longshot, MPTO says 16.00% and OR says 13.75%. That is 2.25 percentage
points, or a fair price of +525 against +627 — a gap far larger than most edges
anyone is betting into.

The lesson is not that one method is right. It is that on a longshot your
choice of devig method matters *more* than the edge you think you have found,
so it is worth knowing which assumption you are making rather than accepting a
default.

## Which Should You Use?

- **MPTO** is the safest default — simple, well understood, and widely used
- **Shin** is theoretically grounded and preferred by academics, but adds
  nothing over EM unless the market has three or more outcomes
- When methods **agree**, be careful about reading that as confirmation —
  check first that they are not the same function, as Shin and EM are here
- When methods **disagree** significantly, the market is lopsided and the
  choice is doing real work. Investigate rather than take the default.

The Devig Calculator runs all five side by side so you can see the spread.
