---
status: accepted
date: 2026-04-20
tags: [agent, feedback, semantics, core]
---

# Feedback is Side-Channel, Not Observations

## Context

In the Agent trait, there are two ways to pass information to the agent: `step(obs)` and `feedback(fb)`. The distinction needed to be formalized.

## Decision

- **Observations** (via `step`): inputs that drive the next action. User messages, tool results, environment states.
- **Feedback** (via `feedback`): side-channel signals that inform but don't directly drive the next step. Scores, rewards, reflection critiques, human ratings.

## Consequences

- In a standard LLM conversation, tool results are observations passed to `step()`, not feedback. `feedback()` may not be called at all during a basic conversation.
- In a Reflexion strategy, a reflection agent evaluates the primary agent's output, and the critique is passed via `feedback()` to the primary agent. The Gyre orchestrates this.
- In RL, rewards are feedback. Next environment states are observations.
- This distinction must be clearly documented — users may initially expect tool results to go through `feedback()`.

## Rationale

Merging feedback into observations would lose semantic information. A reward signal has different implications than the next environment state — the agent should know which is which. Separate channels preserve this.
