---
status: accepted
date: 2026-04-20
tags: [agent, control-flow, core]
---

# StepResult Enum Replaces should_stop Method

## Context

The original Agent trait had `fn should_stop(&self) -> bool` as a side-channel termination signal. This created ambiguity: the Gyre had to check a separate method after every step to know if the agent wanted to stop.

## Decision

Replace `should_stop` with the return type of `step`:

```rust
pub enum StepResult<A> {
    Continue(A),  // action produced, loop continues
    Done(A),      // final action, agent signals completion
}
```

`step` returns `Result<StepResult<Action>, Error>`.

## Consequences

- Termination intent is per-step, not a separate query — no timing ambiguity.
- Both variants carry an action, so the final step always produces output.
- The Gyre pattern-matches on the result — explicit control flow, no hidden state.
- The Gyre can also terminate externally (timeout, max turns, environment done) regardless of StepResult.
