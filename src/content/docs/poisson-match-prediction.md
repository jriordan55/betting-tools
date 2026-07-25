---
title: "Poisson Match Prediction: Modeling Scorelines, Spreads, and Totals"
date: "2025-12-15"
excerpt: "How to use Poisson distributions to model match outcomes — generating scoreline probabilities, moneyline odds, spread prices, and totals."
tags: ["poisson", "match-prediction", "modeling", "simulation"]
---

## Why Poisson?

Goals in soccer, runs in baseball, goals in hockey — these are low-frequency events occurring over a fixed period. The Poisson distribution models exactly this: the count of independent events when you know the average rate.

$$
P(X = k) = \frac{\lambda^k e^{-\lambda}}{k!}
$$

A team expected to score 1.4 goals has a 24.7% chance of scoring exactly 1, a 17.3% chance of exactly 2, and a 24.7% chance of being shut out. One parameter, the full range of outcomes.

## Building the Score Matrix

The key assumption is **independence**: each team's scoring follows its own Poisson process. This lets us build a joint probability matrix where each cell is the product of two marginal probabilities:

$$
P(\text{Home}=h, \text{Away}=a) = \frac{\lambda_H^h e^{-\lambda_H}}{h!} \cdot \frac{\lambda_A^a e^{-\lambda_A}}{a!}
$$

For $\lambda_H = 1.6$ and $\lambda_A = 1.1$, the matrix (as percentages):

|  | **0** | **1** | **2** | **3** |
|--|-------|-------|-------|-------|
| **0** | 6.7 | 7.3 | 4.0 | 1.5 |
| **1** | 10.7 | 11.7 | 6.4 | 2.4 |
| **2** | 8.5 | 9.4 | 5.1 | 1.9 |
| **3** | 4.6 | 5.0 | 2.8 | 1.0 |

Every cell is a tradeable scoreline. Sum the right cells and you get any market.

## Deriving Market Probabilities

**Moneyline** — Sum all cells where home > away (home win), away > home (away win), and the diagonal (draw). For sports without draws, the draw probability redistributes proportionally.

**Spreads** — For a -1.5 spread, sum all cells where $h - a > 1.5$. The complement gives the other side.

**Totals** — For over 2.5, sum all cells where $h + a > 2.5$.

One model, every market. This is the power of the matrix approach.

## Strengths

Poisson is the standard model for soccer and performs well for hockey and baseball — any sport where scoring events are relatively rare and roughly independent. The model requires just two inputs (expected goals for each team) and produces a complete probability distribution over all outcomes.

## Limitations

- **Independence** — in practice, goals are not fully independent. A trailing team pushes forward, affecting both teams' rates
- **Thin tails** — Poisson constrains variance to equal the mean. Real scoring often has variance exceeding the mean, making blowouts more likely than the model predicts
- **Draw inflation** — soccer draws occur more often than independent Poisson processes predict, likely due to tactical effects

## Practical Tips

- Use expected goals (xG) as inputs rather than raw goal totals — xG is less noisy
- Check your model against the totals market for calibration — if your model and the book agree, there is likely no edge
- Compare Poisson predictions to the Negative Binomial model for the same inputs — when they disagree, the truth is likely between them
- The Poisson Match Predictor builds the full score matrix and derives moneyline, spread, and total probabilities from your inputs
