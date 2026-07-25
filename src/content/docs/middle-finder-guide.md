---
title: "Finding Middles: When Two Books Give You a Free Shot"
date: "2025-09-15"
excerpt: "How to calculate the probability and expected value of landing in the middle when you hold opposite positions at different numbers."
tags: ["middles", "arbitrage", "expected-value", "strategy"]
---

## What Is a Middle?

A middle occurs when you hold opposite sides of the same game at different numbers, creating a gap where both bets win. For example:

- Book A: Celtics -3.5 at -110
- Book B: Lakers +5.5 at -110

If the Celtics win by exactly 4 or 5, both bets cash. You bet the favorite at a tight spread and the underdog at a wide spread, and the gap between 3.5 and 5.5 is your middle window.

## The Probability Calculation

Assuming the scoring margin follows a normal distribution $M \sim \mathcal{N}(\mu, \sigma^2)$, the probability of landing in the gap between thresholds $a$ and $b$ (where $a < b$) is:

$$
P(\text{middle}) = \Phi\left(\frac{b - \mu}{\sigma}\right) - \Phi\left(\frac{a - \mu}{\sigma}\right)
$$

In our example with $a = 3.5$, $b = 5.5$, $\mu \approx 4.5$, and $\sigma = 13.5$ (NFL):

$$
P(\text{middle}) = \Phi\left(\frac{5.5 - 4.5}{13.5}\right) - \Phi\left(\frac{3.5 - 4.5}{13.5}\right) \approx 5.9\%
$$

About a 5.9% chance both bets win.

## Expected Value of a Middle

A middle has three possible outcomes:

| Outcome | Result |
|---------|--------|
| Both win (middle hits) | Profit from both sides |
| Side A wins only | Win bet A, lose bet B |
| Side B wins only | Win bet B, lose bet A |

The EV sums across all outcomes, weighted by probability:

$$
\text{EV} = P_{\text{mid}} \cdot (\text{profit}_A + \text{profit}_B) + P_A \cdot (\text{profit}_A - \text{stake}_B) + P_B \cdot (\text{profit}_B - \text{stake}_A)
$$

When both single-side outcomes result in a small loss (you win one side but lose the other minus vig), the middle provides a probabilistic bonus that can push overall EV positive.

## Middles vs. Standard Arbitrage

Pure arbitrage locks in guaranteed profit regardless of outcome. Middles accept a small expected loss on most outcomes in exchange for a large bonus when the result lands in the gap. Think of it as a low-cost lottery ticket attached to an otherwise near-breakeven position.

The advantage: middles require far less line discrepancy than true arbs, so opportunities appear more frequently.

## When Are Middles Worth It?

Key factors:

- **Gap size** — wider gaps mean higher middle probability. A 2-point gap on key numbers in football is much more valuable than a 1-point gap on non-key numbers
- **Key numbers** — NFL margins cluster around 3, 7, 6, and 10. Middles spanning these numbers have outsized probability
- **Juice paid** — each side costs vig. At -110/-110, you lose about 4.5% on the guaranteed portion
- **True center location** — if the expected margin sits inside the gap, middle probability increases significantly

A rough threshold: middles are attractive when the double-win payout times the middle probability exceeds the expected vig drag on single-win outcomes.

## Practical Tips

- Totals middles work the same way: Over 221.5 at one book, Under 224.5 at another
- Always calculate EV, not just gap probability — a wide middle at bad juice can still be -EV
- The Middle Finder handles both spread and total middles and automatically detects traps (where the gap works against you instead of for you)
