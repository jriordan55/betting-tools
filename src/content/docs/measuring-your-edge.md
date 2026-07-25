---
title: "Measuring Your Edge: Expected Value and Closing Line Value"
date: "2025-05-15"
excerpt: "Two essential metrics for evaluating your betting performance — how to calculate EV for any bet and why CLV is the best predictor of long-term profit."
tags: ["expected-value", "clv", "edge", "strategy"]
---

## Expected Value

Expected value is the single most important concept in sports betting. It answers a simple question: if you made this exact bet thousands of times, how much would you expect to win or lose per dollar wagered?

$$
\text{EV} = p \cdot b - (1 - p)
$$

where $p$ is the true win probability and $b = d - 1$ is the net profit per unit at decimal odds $d$.

### Worked Example

You estimate a team has a 55% chance to win. The best available odds are +105 (decimal 2.05):

$$
\text{EV} = 0.55 \times 1.05 - 0.45 = 0.5775 - 0.45 = +0.1275
$$

That is +12.75 cents per dollar bet — a strong edge. On a $100 bet, your expected profit is $12.75.

### The Edge Threshold

A bet has positive EV when your estimated probability exceeds the implied probability from the odds:

$$
p_{\text{true}} > \frac{1}{d} = p_{\text{implied}}
$$

At +105 (implied 48.8%), any true probability above 48.8% creates a +EV bet. The gap between your true probability and the implied probability is your **edge**.

## Why +EV Is Necessary but Not Sufficient

Positive EV means you will profit in the long run — but the long run can be very long. A bettor with a 3% edge on -110 lines will have losing months, possibly losing quarters. Variance is real, and small edges take thousands of bets to converge.

This is why bet sizing (Kelly Criterion), bankroll management, and sample size all matter. EV tells you *whether* to bet. It does not tell you *how much* or guarantee any short-term result.

## Closing Line Value

CLV measures how much better your odds were compared to where the line closed. If you bet Team A at -105 and the line closes at -115, you got 10 cents of closing line value.

### Why CLV Matters

The closing line is the most efficient price the market produces. It incorporates all available information — public betting, sharp action, injury news, weather. Research consistently shows that CLV is the single best predictor of long-term profitability.

A bettor who consistently beats the closing line is almost certainly a long-term winner, even during a losing stretch. Conversely, a bettor on a hot streak who is not beating the close is likely running above expectation.

### Calculating CLV

There are two common methods:

**Odds-based CLV** compares implied probabilities directly:

$$
\text{CLV} = p_{\text{closing}} - p_{\text{bet}}
$$

If you bet at -105 (implied 51.22%) and the line closed at -115 (implied 53.49%):

$$
\text{CLV} = 53.49\% - 51.22\% = +2.27\%
$$

**Return-based CLV** compares what you would have won at your odds vs. the closing odds, showing the percentage return advantage.

### The Relationship Between EV and CLV

If the closing line is efficient (a reasonable assumption for liquid markets), then CLV *is* your edge. Beating the close by 2% means your bets have roughly 2% expected value.

This makes CLV a more reliable metric than raw profit/loss for evaluating your process. Profits can be noisy in small samples. CLV converges much faster because it measures your ability to find value before the market corrects.

## Practical Tips

- Track CLV for every bet, not just profit/loss — it is the earlier signal of whether your process works
- A positive CLV of 1-3% on average is excellent and sustainable
- If your CLV is negative but you are profitable, you are likely running hot — adjust expectations
- Use the [Expected Value](/calculators/expected-value) and [CLV Calculator](/calculators/clv-calculator) to evaluate your bets
