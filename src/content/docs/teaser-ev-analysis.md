---
title: "Teaser EV: When Buying Points Is Worth It"
date: "2025-10-15"
excerpt: "How to calculate cover probabilities and expected value for football teasers using CDF analysis, and why key numbers matter."
tags: ["teasers", "key-numbers", "NFL", "expected-value", "strategy"]
---

## What Is a Teaser?

A teaser lets you move the spread in your favor on two or more legs in exchange for reduced payout odds. Buy 6 points on a 2-team NFL teaser and the book might pay -120 instead of the +260 a parlay would offer.

On the surface this looks like a bad trade — giving up massive payout for a modest spread adjustment. And often that intuition is correct. But the math depends entirely on how much cover probability those extra points actually buy you.

## The Core Question: How Much Probability Do You Gain?

Not all teaser points are created equal. Moving a spread from -13.5 to -7.5 covers a range of margins that are relatively uncommon. Moving from -7.5 to -1.5 covers margins that occur far more frequently, because NFL scoring clusters around certain numbers.

The EV of a teaser comes down to a simple comparison:

- **Combined cover probability** of all legs at their teased spreads
- **Break-even probability** implied by the book's teaser payout odds

If your combined probability exceeds break-even, the teaser is +EV.

## Calculating Cover Probability

### Step 1: Back Out the True Line

Each teaser leg starts as a normal spread bet. From the book's spread and odds, we extract the implied true line using the inverse normal CDF:

$$
\mu = s - \sigma \cdot \Phi^{-1}(p_{\text{cover}})
$$

where $s$ is the posted spread, $p_{\text{cover}}$ is the implied cover probability from the odds, and $\sigma$ is the sport-specific standard deviation (about 14.2 for NFL, 22.6 for NCAAF).

### Step 2: Cover Probability at the Teased Spread

With the true line $\mu$ in hand, the probability of covering the teased spread $s_t = s + \text{teaser points}$ is:

$$
P(\text{cover}) = \Phi\left(\frac{s_t - \mu}{\sigma}\right)
$$

For a -7.5 spread at -110 teased by 6 to -1.5:

$$
\mu \approx -7.5 - 14.2 \cdot \Phi^{-1}(0.524) \approx -8.35
$$

$$
P(\text{cover at } {-1.5}) = \Phi\left(\frac{-1.5 - (-8.35)}{14.2}\right) \approx 68.6\%
$$

### Step 3: Combined Probability and EV

For independent legs, the combined probability is the product:

$$
P_{\text{combined}} = \prod_{i=1}^{n} P_i(\text{cover})
$$

The break-even probability for a teaser paying at American odds $d$:

$$
P_{\text{BE}} = \frac{|d|}{|d| + 100} \quad \text{(for negative odds)}
$$

Expected value as a percentage of breakeven:

$$
\text{EV\%} = \frac{P_{\text{combined}} - P_{\text{BE}}}{P_{\text{BE}}} \times 100
$$

## Key Numbers: Why Some Legs Are Better Than Others

NFL games don't land on every margin equally. Due to the scoring structure (field goals = 3, touchdowns + PAT = 7), final margins of **3** and **7** occur far more frequently than neighboring numbers. The margin lands on exactly 3 about 15% of the time, and exactly 7 about 9% of the time.

This means a teaser leg that moves through 3 or 7 gains more cover probability per point than one that doesn't. Secondary key numbers **10** (field goal + touchdown) and **14** (two touchdowns) matter too, though less dramatically.

The calculator shows which key numbers each leg crosses, so you can see at a glance whether your teaser points are landing in high-value territory or being spent on margins where games rarely land.

### Examples

| Spread | Teased (+6) | Key #s Crossed | Value |
|--------|-------------|----------------|-------|
| -7.5 | -1.5 | 3, 7 | High — crosses both primary key numbers |
| -5.5 | +0.5 | 3 | Good — crosses 3 and gets past zero |
| -3.5 | +2.5 | 3 (barely) | Moderate — crosses 3 but not 7 |
| +2.5 | +8.5 | 3, 7 | High — crosses both going the other direction |
| -13.5 | -7.5 | 10 | Low — only crosses 10, misses the big two |

## Common Teaser Payouts

| Format | Typical Payout | Break-even |
|--------|---------------|------------|
| 2-team, 6 pts | -120 to -110 | 54.5% to 52.4% |
| 2-team, 6.5 pts | -130 to -120 | 56.5% to 54.5% |
| 2-team, 7 pts | -140 to -130 | 58.3% to 56.5% |
| 3-team, 6 pts | +150 to +180 | 40.0% to 35.7% |

Shopping for the best teaser payout across books is critical. The difference between -110 and -130 on a 2-team teaser shifts break-even by 4 percentage points — often the difference between +EV and -EV.

## NFL vs. NCAAF

College football has the same key numbers but a much larger scoring standard deviation (~22.6 vs ~14.2). The wider distribution means each teaser point buys less relative probability. The same 6-point teaser that might be +EV in the NFL is typically -EV in NCAAF because the probability gain is smaller relative to the payout discount.

## Limitations

- **Independence assumption**: The calculator assumes legs are independent. Correlated legs (e.g., same-game legs or games with shared factors) violate this — use the SGP Analyzer for correlated bets.
- **Normal distribution**: Real scoring margins have slightly heavier tails and point masses at key numbers. The CDF model is a close approximation but not exact.
- **Line accuracy**: The true line estimate depends on the input odds. Stale or soft lines produce unreliable results. Pinnacle closing lines give the best input.

## Using the Calculator

Enter each leg's spread and odds as quoted by the book, select your teaser size, and enter the book's teaser payout odds. The calculator shows per-leg cover probabilities, which key numbers are crossed, and whether the combined probability exceeds break-even. Positive EV means the teaser is worth taking at that payout; negative means the book's discount more than offsets the extra points.
