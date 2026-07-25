---
title: "Hold, Vig, and Arbitrage: Understanding the Book's Edge"
date: "2025-04-15"
excerpt: "How sportsbooks make money through hold and vig, how to compare lines across books, and how to find guaranteed-profit arbitrage opportunities."
tags: ["hold", "vig", "arbitrage", "line-shopping"]
---

## What Is Hold?

Hold is the sportsbook's built-in margin — the percentage of total handle they expect to keep regardless of outcome. For a two-sided market, it is calculated from the implied probabilities:

$$
\text{Hold} = p_{\text{implied},1} + p_{\text{implied},2} - 1
$$

A standard -110/-110 market has implied probabilities of 52.38% each, so the hold is $52.38 + 52.38 - 100 = 4.76\%$. This means for every $100 wagered across both sides, the book expects to keep $4.76.

## Why Hold Varies

Not all markets carry the same hold. Liquid, high-volume markets (NFL sides, major soccer matches) often have holds of 3-5%. Less liquid markets (props, lower leagues) can have holds of 8-15% or more. The hold tells you how much the book is charging for the privilege of betting.

Knowing the hold also lets you derive no-vig fair odds — what the line would be if the book charged zero margin. This is useful for finding true probabilities and comparing across books.

## Comparing Vig Across Books

Different books price the same market differently. One book might offer -108/-112, another -110/-110, and a third -105/-115. They all have different hold percentages:

| Book | Side A | Side B | Hold |
|------|--------|--------|------|
| Book 1 | -108 | -112 | 4.3% |
| Book 2 | -110 | -110 | 4.8% |
| Book 3 | -105 | -115 | 4.4% |

The lowest-vig book is not always the one with the best price for your side. You might find the best odds on Side A at Book 3 (-105) even though Book 1 has the lowest overall hold. This is why sharp bettors compare both total vig and individual line prices.

## How Arbitrage Works

An arbitrage opportunity exists when the combined implied probabilities across different books sum to less than 100%:

$$
\frac{1}{d_1} + \frac{1}{d_2} < 1
$$

where $d_1$ and $d_2$ are the best decimal odds for each side from any book. When this holds, you can bet both sides and guarantee a profit.

### Optimal Stake Allocation

To guarantee equal profit regardless of outcome, allocate stakes inversely proportional to the odds:

$$
s_i = \frac{\text{Total Investment}}{d_i \cdot \sum_j \frac{1}{d_j}}
$$

### Worked Example

Book A offers Team X at +130 (decimal 2.30). Book B offers Team Y at -120 (decimal 1.833).

$$
\frac{1}{2.30} + \frac{1}{1.833} = 0.435 + 0.546 = 0.981
$$

Since $0.981 < 1$, this is an arb with a guaranteed return of $1/0.981 - 1 = 1.94\%$.

With a \$1,000 total investment: bet \$443 on Team X at Book A and \$557 on Team Y at Book B. If Team X wins, you collect $443 \times 2.30 = \$1019$. If Team Y wins, you collect $557 \times 1.833 = \$1021$. Either way, you profit roughly \$19-21.

## Practical Limits of Arb Betting

Arbitrage sounds like free money, and mathematically it is. But in practice:

- **Books limit arb bettors** — accounts that consistently bet both sides get restricted quickly
- **Line movement** — by the time you place the second leg, the odds may have moved
- **Capital requirements** — a 2% arb on $1,000 yields $20. You need significant capital for meaningful returns
- **Timing** — opportunities are fleeting, often lasting minutes

Arb betting is best used as an occasional supplement, not a primary strategy. The real value of understanding arbitrage is that it sharpens your sense of fair value.

## Practical Tips

- Always check the hold before placing a bet — if the hold is 10%+, you are paying a steep tax
- Line-shop across at least 3-4 books for every bet
- Use the [Hold Calculator](/calculators/hold-calculator), [Vig Comparison](/calculators/vig-comparison), and [Arbitrage Calculator](/calculators/arbitrage-calculator) to find the best prices
