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

## When Do They Differ?

For balanced markets (e.g., -110/-110), all five methods produce nearly identical results. Differences emerge in **unbalanced markets**:

| Market | EM | MPTO | Shin | OR | LOG |
|--------|-----|------|------|-----|-----|
| -300/+250 | 72.50% | 73.17% | 73.35% | 73.25% | 73.21% |
| -110/-110 | 50.00% | 50.00% | 50.00% | 50.00% | 50.00% |
| +500/-700 | 14.64% | 14.63% | 14.50% | 14.57% | 14.60% |

The differences are small (typically < 1%), but when you're making decisions at the margin, even 0.5% matters.

## Which Should You Use?

- **MPTO** is the safest default — it's simple, well-understood, and widely used
- **Shin** is theoretically grounded and preferred by academics
- When methods **agree**, you can be more confident in the fair probability
- When methods **disagree** significantly, the market may be unusual — investigate further

The Bettor Calculator Devig Calculator shows all five methods side-by-side so you can compare.
