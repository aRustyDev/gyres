---
status: accepted
date: 2026-04-20
tags: [agent, feedback, performance, core]
---

# Feedback Method is Synchronous

## Context

The `feedback` method delivers side-channel signals (scores, rewards, reflections) to agents. The question was whether it should be async.

## Decision

`fn feedback(&self, fb: &Self::Feedback)` — synchronous, no future returned.

## Consequences

- LLM agents: feedback is `history.write().push(fb.clone())` — nanoseconds. No async overhead.
- RL agents: buffer rewards in feedback, process them in the next `step()` call (which is already async). Standard RL pattern (PPO, SAC, DQN all batch updates).
- Gyres don't need `.await` on feedback calls — simpler loop code.
- RL agents needing immediate async processing (rare: online learning with remote policy server) can spawn a background task in feedback and await it in step.

## Alternatives Considered

- **Async feedback**: Would add Future allocation and executor scheduling overhead to every feedback call. The 99% case (Vec::push, buffer append) doesn't benefit. Rejected because it penalizes the common case for an uncommon scenario.
