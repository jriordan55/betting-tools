---
title: "The Kelly Criterion: Optimal Bet Sizing for Sports Bettors"
date: "2025-02-10"
excerpt: "A practical guide to the Kelly Criterion — the mathematically optimal formula for sizing your bets based on edge and bankroll."
tags: ["kelly", "bankroll", "strategy"]
---

## What Is the Kelly Criterion?

The Kelly Criterion is a formula that tells you what fraction of your bankroll to bet, given your edge. It maximizes the expected logarithmic growth of your bankroll — meaning it grows your money as fast as possible while managing risk of ruin.

## The Formula

For a bet with decimal odds $d$ and true win probability $p$:

$$
f^* = \frac{bp - q}{b}
$$

where:
- $b = d - 1$ (the profit per unit bet)
- $p$ = probability of winning
- $q = 1 - p$ = probability of losing
- $f^*$ = fraction of bankroll to bet

### Example

You find a bet at +150 (decimal 2.50) and estimate a 45% chance of winning:

$$
b = 2.50 - 1 = 1.50
$$
$$
f^* = \frac{1.50 \times 0.45 - 0.55}{1.50} = \frac{0.675 - 0.55}{1.50} = \frac{0.125}{1.50} = 8.33\%
$$

With a \$10,000 bankroll, you'd bet \$833.

## Why Full Kelly Is Dangerous

Full Kelly maximizes growth rate but creates enormous variance. In practice:

- A **50% drawdown** is almost certain over a long career
- The formula assumes you know the **exact** true probability — you don't
- Overestimating your edge by even a small amount leads to catastrophic overbetting

## Fractional Kelly

Most sharp bettors use a fraction of Kelly:

| Strategy | Fraction | Growth Rate | Variance |
|----------|----------|-------------|----------|
| Full Kelly | 100% | Maximum | Very high |
| 3/4 Kelly | 75% | ~94% of max | Moderate |
| Half Kelly | 50% | ~75% of max | Low |
| Quarter Kelly | 25% | ~44% of max | Very low |

**Half Kelly** is the most popular choice. You sacrifice 25% of optimal growth but reduce variance dramatically.

## The Edge Requirement

Kelly only recommends a bet when you have positive expected value. If $f^* \leq 0$, don't bet:

$$
f^* > 0 \iff bp > q \iff p > \frac{1}{d}
$$

In other words, your estimated win probability must exceed the implied probability from the odds.

## Practical Guidelines

1. **Never bet full Kelly** unless you have extremely accurate probability estimates
2. **Half Kelly is a good default** — it captures most of the growth with much less risk
3. **Track your CLV** (closing line value) to validate your probability estimates over time
4. **Reduce Kelly fraction** when uncertain — if you're unsure of your edge, bet smaller
5. **Don't Kelly multiple correlated bets** — the formula assumes independent events

## The Connection to Expected Value

Kelly and EV are related but different:

$$
\text{EV} = p \cdot b - q = b \cdot f^*
$$

A positive EV bet always has positive Kelly. But Kelly also accounts for your **bankroll** — it tells you not just *whether* to bet, but *how much*.

A 1% edge on a -110 bet suggests a small Kelly fraction. A 10% edge on the same bet suggests a much larger one. EV alone doesn't capture this.
