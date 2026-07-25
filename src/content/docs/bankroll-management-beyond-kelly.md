---
title: "Bankroll Management Beyond Kelly: Hedging and Risk of Ruin"
date: "2025-06-15"
excerpt: "When to hedge an existing bet to lock in profit, and how to estimate your probability of going broke with Monte Carlo simulation."
tags: ["hedging", "bankroll", "risk-of-ruin", "strategy"]
---

## When and Why to Hedge

Hedging means placing a second bet on the opposite side of an existing wager to guarantee a profit or limit a loss. It comes up most often with futures and parlays — situations where your original bet has increased in value.

The core tradeoff: hedging trades potential upside for certainty. A full hedge guarantees the same profit regardless of outcome. A partial hedge reduces downside while keeping some upside exposure.

## The Hedge Formula

You have an existing bet with potential payout $P$ (total return including stake). You can hedge by betting the other side at decimal odds $d_{\text{hedge}}$. To lock in equal profit on both outcomes:

$$
\text{Hedge Stake} = \frac{P}{d_{\text{hedge}}}
$$

Your guaranteed profit is:

$$
\text{Profit} = P - \text{Original Stake} - \text{Hedge Stake}
$$

### Worked Example

You bet \$100 on the Chiefs at +2000 (decimal 21.00) before the season. They make the Super Bowl. The opponent is -150 (decimal 1.667).

Potential payout if Chiefs win: $\$100 \times 21.00 = \$2100$.

$$
\text{Hedge Stake} = \frac{\$2100}{1.667} = \$1260
$$

If the Chiefs win: collect \$2,100, lose the \$1,260 hedge — net \$740. If they lose: collect $\$1260 \times 1.667 = \$2100$ from the hedge, minus \$100 original and \$1,260 stake — also \$740. Either way, \$740 profit from a \$100 bet.

## When Hedging Makes Sense

Hedging is technically -EV — you pay vig on the second bet. But it is the right call when:

1. **The guaranteed amount is significant** — utility of money is not linear
2. **Your original bet was recreational** — locking in a 7x return on a long-shot future is sensible
3. **You are overexposed** — if one bet is too large a share of your bankroll, hedging is risk management

Sharp bettors generally avoid hedging standard plays. But for large futures payouts, hedging is a tool worth understanding.

## Risk of Ruin

Risk of ruin is the probability that your bankroll hits zero (or drops below your minimum bet size) over a given number of bets. Even with a genuine edge, aggressive bet sizing can lead to ruin through an unlucky sequence.

## Why Monte Carlo Simulation?

Analytical formulas for risk of ruin exist for simple cases, but they assume fixed odds, fixed bet sizes, and independent bets. Monte Carlo simulation handles the general case by running thousands of simulated bankroll paths and counting how many go bust.

Each simulation path starts with your bankroll, resolves each bet as win or loss, and tracks whether the bankroll drops below the minimum. After thousands of paths, the fraction that went bust is your estimated risk of ruin.

## How Edge and Sizing Affect Survival

The two variables that matter most:

**Edge** — A 3% edge on -110 lines has dramatically lower ruin probability than a 1% edge, all else equal.

**Bet size relative to bankroll** — This is the lever you control:

| Bet Size (% of bankroll) | Approximate Ruin Risk |
|--------------------------|----------------------|
| 1% | Near zero |
| 2.5% | Low |
| 5% | Moderate |
| 10% | High |

The relationship is nonlinear. Doubling your bet size more than doubles your ruin risk.

## The Connection to Kelly

Full Kelly maximizes long-term growth but comes with a near-certain 50% drawdown at some point. This is why most sharp bettors use fractional Kelly — typically half Kelly or less.

Risk of ruin simulation lets you stress-test your strategy before committing real money. You can see exactly how your bet size, edge, and bankroll interact over hundreds or thousands of bets.

## Practical Tips

- Never bet more than 5% of your bankroll on a single play, even with a large edge
- Use simulation to stress-test before you start betting
- Hedge selectively, not habitually — only when the guaranteed amount justifies the EV cost
- Use the [Hedge Calculator](/calculators/hedge-calculator) and [Risk of Ruin](/calculators/risk-of-ruin) calculator to run the numbers
