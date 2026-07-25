---
title: "Understanding Betting Odds: Formats, Conversion, and Parlays"
date: "2025-03-15"
excerpt: "A complete guide to American, decimal, and fractional odds — how to convert between them, extract implied probabilities, and calculate parlay payouts."
tags: ["odds", "probability", "parlays", "basics"]
---

## Three Odds Formats

Every sportsbook displays odds in one of three formats. They all encode the same information — the ratio of profit to stake — just differently.

**American odds** use a +/- system anchored to $100. A favorite at -150 means you risk $150 to profit $100. An underdog at +150 means you risk $100 to profit $150.

**Decimal odds** express total return per dollar wagered. Decimal 2.50 means a $100 bet returns $250 total ($150 profit + $100 stake). Decimal odds are always greater than 1.

**Fractional odds** show the profit-to-stake ratio directly. 3/2 means $3 profit for every $2 staked — the same as +150 or decimal 2.50.

## Converting Between Formats

From American to decimal:

$$
d = \begin{cases} 1 + \frac{|A|}{100} & \text{if } A > 0 \\ 1 + \frac{100}{|A|} & \text{if } A < 0 \end{cases}
$$

From decimal to implied probability:

$$
p_{\text{implied}} = \frac{1}{d}
$$

A line at -150 converts to decimal 1.667, which implies a $1/1.667 = 60\%$ win probability. But that 60% includes the book's vig — the true probability is lower.

## Break-Even Win Rate

The implied probability is also your break-even win rate. At -110 (decimal 1.909), you need to win $1/1.909 = 52.4\%$ of the time just to break even. This is why -110/-110 markets are the book's bread and butter — both sides need 52.4%, totaling 104.8%.

That extra 4.8% is the overround, and it is the book's margin on every dollar wagered.

## Parlay Math

A parlay combines multiple independent bets. The combined decimal odds are the product of each leg's decimal odds:

$$
d_{\text{parlay}} = d_1 \times d_2 \times \cdots \times d_n
$$

A 3-leg parlay at -110, -110, -110:

$$
d_{\text{parlay}} = 1.909 \times 1.909 \times 1.909 = 6.96
$$

The implied probability of hitting all three legs (assuming independence):

$$
p_{\text{parlay}} = p_1 \times p_2 \times p_3 = 0.524^3 = 14.4\%
$$

## The Compounding Vig Problem

Here is the catch with parlays. Each leg carries vig, and vig compounds multiplicatively. A single -110 bet has about 4.8% overround. A 3-leg parlay at -110 per leg:

$$
\text{Fair odds} = 2.00^3 = 8.00 \quad \text{vs. actual } 1.909^3 = 6.96
$$

The effective overround on the parlay is $8.00/6.96 - 1 = 14.9\%$. The book's margin nearly triples from a single bet to a 3-legger. This is why sportsbooks promote parlays so aggressively.

## Practical Tips

- Always think in implied probability, not odds format — it makes comparisons across books and formats instant
- Parlays are not inherently bad, but understand that the vig scales with the number of legs
- Use the [Odds Converter](/calculators/odds-converter), [Sharp Implied](/calculators/sharp-implied), and [Parlay Calculator](/calculators/parlay-calculator) to run these conversions instantly
