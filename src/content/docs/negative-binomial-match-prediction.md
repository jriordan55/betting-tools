---
title: "Negative Binomial Match Prediction: When Poisson Isn't Enough"
date: "2026-01-05"
excerpt: "Why some sports need an overdispersed scoring model — and how the Negative Binomial distribution captures blowouts that Poisson misses."
tags: ["negative-binomial", "match-prediction", "overdispersion", "modeling"]
---

## The Poisson Constraint

Poisson is elegant, but it enforces a strict rule: the variance must equal the mean. If a team averages 2.5 goals, Poisson says the variance is also 2.5. In many sports, scoring variance significantly exceeds the mean. This is called **overdispersion**.

Consider NFL scoring. Teams average around 22 points per game, but blowouts (45-3) and shootouts (42-38) happen more often than Poisson predicts. The same pattern appears in basketball, rugby, and even some hockey matchups.

When variance exceeds the mean, Poisson underestimates extreme outcomes. Your tail probabilities are wrong, your spread prices are biased, and your totals are too tight.

## The Negative Binomial Solution

The Negative Binomial (NB) distribution adds a dispersion parameter $r$ that decouples variance from the mean:

$$
\text{Var} = \mu + \frac{\mu^2}{r}
$$

The extra $\mu^2 / r$ term is always positive, so NB variance always exceeds the mean. Lower $r$ means more overdispersion (fatter tails). As $r \to \infty$, the extra term vanishes and NB converges to Poisson.

The PMF:

$$
P(X = k) = \binom{k + r - 1}{k} \cdot p^r \cdot (1-p)^k
$$

where $p = r/(r + \mu)$.

## The Gamma-Poisson Interpretation

There is an intuitive way to understand NB. Instead of assuming every game has the same scoring rate $\lambda$, suppose the rate itself varies — drawn from a Gamma distribution before each game. The resulting distribution of scores is exactly the Negative Binomial.

This makes physical sense. A team's scoring rate depends on matchups, game flow, weather, and randomness. Some games are low-scoring grinds, others are track meets. NB captures this natural game-to-game variation.

## Poisson vs. NB: A Comparison

Take two teams with expected scoring rates of 2.8 and 2.2. How Poisson and NB ($r = 5$) compare:

| Outcome | Poisson | NB ($r=5$) |
|---------|---------|------------|
| 0 goals (home) | 6.1% | 8.5% |
| 5+ goals (home) | 8.1% | 12.3% |
| Total over 6.5 | 22.4% | 27.1% |
| Home by 4+ | 5.8% | 8.2% |

NB shifts probability from the center to both tails. Shutouts and blowouts are both more likely. This directly affects spread and total pricing — especially for large spreads and high totals where tail behavior dominates.

## When to Use NB Over Poisson

- **High-scoring sports** (NFL, NBA, rugby) where game-to-game variance clearly exceeds the mean
- **Blowout-prone matchups** where one team significantly outclasses the other
- **Large spread and total markets** where tail probabilities matter most
- **Any sport where you observe overdispersion** in historical scoring data

For low-scoring sports like soccer, Poisson often performs well enough. But even there, NB can improve predictions for high-powered attacking matchups.

## Estimating the Dispersion Parameter

You can estimate $r$ from historical data using the sample mean and variance:

$$
r = \frac{\mu^2}{\text{Var} - \mu}
$$

If the sample variance is close to the mean, $r$ is large and NB behaves like Poisson. If variance far exceeds the mean, $r$ is small and the NB tails are much fatter.

## Practical Tips

- Start with Poisson, upgrade to NB when your model consistently underestimates blowouts
- Use both models as bounds — when they disagree, the disagreement tells you how sensitive the price is to tail assumptions
- The Negative Binomial Match Predictor lets you control the dispersion parameter directly to see how overdispersion shifts probabilities across every market
