---
title: "Alternate Line Pricing: Fair Odds for Every Spread and Total"
date: "2025-08-15"
excerpt: "How to generate fair prices for alternate spreads and totals from a single market line using probability distributions."
tags: ["alternate-lines", "spreads", "totals", "pricing"]
---

## What Are Alternate Lines?

Sportsbooks offer alternate spreads and totals alongside the main line. If the primary spread is -6.5, you might see alternates at -3.5, -10.5, -13.5, and so on — each at different odds. These let you buy or sell points, adjusting your risk-reward tradeoff.

The problem: books typically price alternates with significantly more vig than the main line. The further you move from the primary number, the wider the margin. But what should these lines actually cost?

## The Mathematical Approach

If you assume the scoring margin (or total) follows a probability distribution, you can derive the fair price of any alternate line from a single known price.

For spreads, model the margin $M$ as normally distributed:

$$
M \sim \mathcal{N}(\mu, \sigma^2)
$$

The fair probability of covering an alternate spread $s$ is:

$$
P(\text{cover}) = 1 - \Phi\left(\frac{s - \mu}{\sigma}\right)
$$

where $\mu$ is the true expected margin (backed out from the primary line and its odds) and $\sigma$ is the sport-specific standard deviation.

## Backing Out the True Center

From the primary market — say, -6.5 at -110 — extract the implied fair probability (after removing vig) and invert the CDF to find $\mu$:

$$
\mu = s_{\text{primary}} - \sigma \cdot \Phi^{-1}(1 - p_{\text{fair}})
$$

Once you have $\mu$, you can price every alternate. Team -3.5? Plug it in. Team -10.5? Same formula. The entire alternate line ladder falls out of the model.

## Totals Work the Same Way

For over/under markets, model the combined score as normally distributed around the expected total. The fair probability of the over at any alternate number:

$$
P(\text{over}) = 1 - \Phi\left(\frac{t - \mu_{\text{total}}}{\sigma_{\text{total}}}\right)
$$

## Finding Value

The power of alternate line pricing is comparison. Generate the fair price for every alternate, then compare to what the book offers. If the book has -3.5 at +170 but your model says the fair price is +155, the book is offering you more than fair value — that is an edge.

Conversely, if the book's alternate is priced tighter than your fair value, the vig is too high to justify the play.

## Practical Tips

- Standard deviations differ by sport and market: NFL game spreads around 13.5, NBA around 12, MLB run lines around 3.5
- Alternates further from the primary line have more model uncertainty — confidence in the fair price decreases
- Key numbers in football (3, 7) create discontinuities that a pure normal model can underestimate — account for this at those specific thresholds
- The Alternate Line Pricer builds the full ladder from a single input, showing fair odds and identifying where the book is overcharging
