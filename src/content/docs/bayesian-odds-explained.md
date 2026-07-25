---
title: "Bayesian Odds: Combining Market Lines With Your Model"
date: "2025-01-15"
excerpt: "How to use Bayesian inference to blend sharp market odds with your own probability model for better betting decisions."
tags: ["bayesian", "probability", "modeling"]
---

## Why Bayesian?

Sports betting markets are efficient — but they're not perfect. If you have a model that produces win probabilities, you face a question: **should you trust the market or your model?**

The answer is both. Bayesian inference gives us a principled way to combine them.

## The Beta-Binomial Framework

We model Team A's true win probability $p$ as a random variable. The market gives us a **prior** belief, and our model provides **evidence**.

### Prior: The Market

Convert the market moneyline to implied probability (removing vig):

$$
p_{\text{fair}} = \frac{p_{\text{implied},A}}{p_{\text{implied},A} + p_{\text{implied},B}}
$$

This becomes a Beta distribution prior:

$$
p \sim \text{Beta}(\alpha_0, \beta_0) \quad \text{where } \alpha_0 = p_{\text{fair}} \cdot N_{\text{market}}, \quad \beta_0 = (1 - p_{\text{fair}}) \cdot N_{\text{market}}
$$

The parameter $N_{\text{market}}$ controls how much you trust the market. Higher values = tighter prior = more trust in the market.

### Evidence: Your Model

Your model says Team A wins with probability $p_{\text{model}}$. We treat this as observing $N_{\text{model}}$ pseudo-trials:

$$
\alpha_{\text{post}} = \alpha_0 + p_{\text{model}} \cdot N_{\text{model}}
$$
$$
\beta_{\text{post}} = \beta_0 + (1 - p_{\text{model}}) \cdot N_{\text{model}}
$$

### Posterior

The posterior win probability is:

$$
p_{\text{posterior}} = \frac{\alpha_{\text{post}}}{\alpha_{\text{post}} + \beta_{\text{post}}}
$$

This is a precision-weighted average. If $N_{\text{market}} = 100$ and $N_{\text{model}} = 50$, the posterior is 2/3 market + 1/3 model.

## Edge Calculation

Once you have a posterior probability, compare it to the available odds:

$$
\text{Edge} = p_{\text{posterior}} - p_{\text{implied}}
$$

$$
\text{EV} = p_{\text{posterior}} \cdot \left(\frac{1}{p_{\text{implied}}} - 1\right) - (1 - p_{\text{posterior}})
$$

If both edge and EV are positive and exceed your threshold, you have a bet.

## Practical Tips

- Start with $N_{\text{market}} = 100$ and $N_{\text{model}} = 20\text{-}50$
- Track your results and adjust the weights over time
- The Bayesian framework naturally handles the case where your model agrees with the market (posterior stays close to the market)
- When your model strongly disagrees, the posterior moves toward your model but is tempered by market wisdom
