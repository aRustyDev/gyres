# gyres-core Spec Discussion — Round 2

## 1E: Does targeting vectorized RL / speculative / MCTS change the calculus?

Yes, significantly. If those aren't hypothetical but *targeted scenarios*, then having two separate traits (`Agent` + `BatchAgent`) creates a problem: every Gyre that wants to support both single and batched execution needs to be generic over two different trait bounds, or you need two Gyre implementations per domain. That's a combinatorial mess.

The deeper issue: `BatchAgent` and `Agent` would share almost identical associated types and semantics. The only difference is "one observation in, one action out" vs "N observations in, N actions out." That's not a different *kind* of agent — it's a different *calling convention* for the same agent.

**Alternative: make `Agent` support both via a single trait.**

```rust
pub trait Agent: Send + Sync {
    type Observation: Send + Sync;
    type Action: Send + Clone;
    type Feedback: Send + Clone;
    type Error: std::error::Error + Send + 'static;

    /// Single-step: produce one action from one observation.
    fn step(
        &self,
        obs: &Self::Observation,
    ) -> impl Future<Output = Result<StepResult<Self::Action>, Self::Error>> + Send;

    /// Batch-step: produce N actions from N observations.
    /// Default implementation calls step() sequentially.
    fn step_batch(
        &self,
        observations: &[Self::Observation],
    ) -> impl Future<Output = Result<Vec<StepResult<Self::Action>>, Self::Error>> + Send {
        async {
            let mut results = Vec::with_capacity(observations.len());
            for obs in observations {
                results.push(self.step(obs).await?);
            }
            Ok(results)
        }
    }

    fn feedback(&self, fb: &Self::Feedback);

    fn reset(&mut self) -> Result<(), Self::Error>;
}
```

Notice the key change: **`step` takes `&self`, not `&mut self`.**

This is the decision point. If we want concurrent/batched/speculative execution, `&self` on `step` is the enabling change. The agent manages its own interior mutability. The tradeoffs:

| | `&mut self` | `&self` |
|---|---|---|
| Sequential LLM loop | Natural, simple | Works (interior mutability is trivial for append-only history) |
| Vectorized RL (64 envs) | Need 64 instances | One instance, `step_batch` or concurrent `step` calls |
| Speculative execution | Can't do it | `join!(step(&obs_a), step(&obs_b))` works |
| MCTS tree search | Can't do it | Branch and explore concurrently |
| Agent impl complexity | Zero — just `&mut self` fields | Need `RwLock`/`Mutex` on mutable state |
| Data race safety | Compile-time guaranteed | Runtime guaranteed (lock contention, but no UB) |

For an LLM agent, the interior mutability cost is minimal:

```rust
pub struct LlmAgent {
    provider: Arc<dyn Provider>,
    history: RwLock<Vec<Message>>,  // was just Vec<Message>
    registry: ToolRegistry,
}

impl Agent for LlmAgent {
    fn step(&self, obs: &Message) -> impl Future<...> + Send {
        async move {
            self.history.write().unwrap().push(obs.clone());
            let messages = self.history.read().unwrap().clone();
            let response = self.provider.complete(&messages, &self.registry.schemas()).await?;
            Ok(StepResult::Continue(response))
        }
    }
}
```

One `RwLock` wrapping the history vec. That's it. The cost is a few nanoseconds per step for the lock — invisible compared to the LLM API call.

For an RL agent doing batched execution:

```rust
pub struct RlAgent {
    policy: RwLock<PolicyNetwork>,
}

impl Agent for RlAgent {
    fn step(&self, obs: &StateVector) -> impl Future<...> + Send {
        async move {
            let action = self.policy.read().unwrap().forward(obs);
            Ok(StepResult::Continue(action))
        }
    }

    fn step_batch(&self, observations: &[StateVector]) -> impl Future<...> + Send {
        async move {
            // GPU-batched forward pass
            let actions = self.policy.read().unwrap().forward_batch(observations);
            Ok(actions.into_iter().map(StepResult::Continue).collect())
        }
    }
}
```

**Subtlety with `feedback`:** If `step` is `&self` (concurrent reads), but `feedback` mutates agent state (writing rewards/history), then `feedback` also needs to be `&self` with interior mutability:

```rust
fn feedback(&self, fb: &Self::Feedback);  // &self, not &mut self
```

This means `feedback` writes go through a lock too. For an LLM agent appending to history, that's fine. For an RL agent accumulating rewards, also fine.

The only method that genuinely needs `&mut self` is `reset` — you want exclusive access when resetting state. `&mut self` on `reset` enforces at compile time that you can't reset while steps are in flight.

**Does this change other decisions?**

Yes — the Gyre trait is affected. If `Agent` is `&self` on `step`, then the Gyre drives the agent via shared reference. The Gyre needs `&mut Agent` only for `reset()`. This means:

```rust
pub trait Gyre<A: Agent>: Send + Sync {
    type Outcome: Send + Clone;

    fn run(
        &self,
        agent: &A,        // &A for steps, not &mut A
        ctx: &GyreContext,
    ) -> impl Future<Output = Result<Self::Outcome, GyreError>> + Send;
}
```

The Gyre doesn't call `reset` — that's the harness executor's job between runs. The Gyre only does `step` + `feedback`, both `&self`.

---

## 3B: Strategy as a type in Gyre

If the Gyre is `&self` but needs adaptive behavior, a `Strategy` type separates "what drives the loop" from "how the loop adapts":

```rust
pub trait Gyre<A: Agent>: Send + Sync {
    type Outcome: Send + Clone;
    type Strategy: Send + Sync;

    fn run(
        &self,
        agent: &A,
        ctx: &GyreContext,
        strategy: &Self::Strategy,
    ) -> impl Future<Output = Result<Self::Outcome, GyreError>> + Send;
}
```

For a conversation gyre:

```rust
pub struct ConversationStrategy {
    pub max_turns: usize,
    pub temperature: f64,
    pub compaction_threshold: usize,
}

impl Gyre<LlmAgent> for ConversationGyre {
    type Outcome = ConversationOutcome;
    type Strategy = ConversationStrategy;

    async fn run(&self, agent: &LlmAgent, ctx: &GyreContext, strategy: &ConversationStrategy) -> ... {
        // Uses strategy.max_turns, strategy.temperature, etc.
    }
}
```

For an RL gyre with adaptive exploration:

```rust
pub struct ExplorationStrategy {
    pub epsilon: f64,           // exploration rate
    pub decay: f64,             // per-episode decay
    pub min_epsilon: f64,
}

impl Gyre<RlAgent> for EpisodeGyre {
    type Strategy = ExplorationStrategy;
    // ...
}
```

The caller can mutate the strategy between runs without touching the Gyre:

```rust
let gyre = EpisodeGyre::new();
let mut strategy = ExplorationStrategy { epsilon: 1.0, decay: 0.995, min_epsilon: 0.01 };

for episode in 0..1000 {
    let outcome = gyre.run(&agent, &ctx, &strategy).await?;
    strategy.epsilon = (strategy.epsilon * strategy.decay).max(strategy.min_epsilon);
}
```

The Gyre stays `&self` (immutable, shareable). The Strategy carries the mutable configuration. Clean separation.

**However** — this adds a type parameter to every Gyre invocation. Alternatives:

```rust
// Option A: Strategy on the trait (shown above)
gyre.run(&agent, &ctx, &strategy).await

// Option B: Strategy baked into the Gyre at construction
let gyre = ConversationGyre::new(ConversationStrategy { max_turns: 10, ... });
gyre.run(&agent, &ctx).await

// Option C: Strategy in GyreContext
ctx.config.set("max_turns", 10);
gyre.run(&agent, &ctx).await
```

Option A is most explicit and type-safe. Option B is simpler but means the Gyre isn't truly stateless (it holds the strategy). Option C is stringly-typed and loses compile-time safety.

**Recommendation: Option A (Strategy as associated type on Gyre)**. It's explicit, the adaptive behavior pattern is natural, and it keeps the Gyre genuinely stateless.

---

## 5B: Extensible typed enums for Turn

Your proposal uses `#[non_exhaustive]` enums for observation/action/feedback types in `gyres-core`, with domain crates adding variants. There's a Rust constraint here: **you can't add variants to an enum defined in another crate.** `#[non_exhaustive]` only means external code must have a wildcard `_` arm when matching — it doesn't allow extending the enum.

So `gyres-core` can't define `ObservationType` and have `gyres-llm` add `LlmObservation(Message)` to it.

**Three approaches that actually work in Rust:**

### Approach 1: Value-based (most flexible, least type-safe)

```rust
// gyres-core
pub struct SerializedTurn {
    pub timestamp: SystemTime,
    pub domain: String,                    // "llm", "rl", etc.
    pub observation: serde_json::Value,
    pub action: serde_json::Value,
    pub feedback: Option<serde_json::Value>,
}
```

Domain crates define typed wrappers with `From`/`TryFrom`. The `domain` field tells the deserializer which typed turn to construct.

### Approach 2: Enum with known variants (your approach, but all variants defined in gyres-core)

```rust
// gyres-core
#[non_exhaustive]
pub enum ObservationType {
    /// Raw JSON for unknown/custom domains
    Raw(serde_json::Value),
    /// LLM: a chat message
    LlmMessage(serde_json::Value),  // gyres-core doesn't know Message, stores as Value
    /// RL: state vector
    RlState(Vec<f64>),
}
```

This works if gyres-core knows the *categories* upfront (LLM, RL) but not the concrete types. The `#[non_exhaustive]` means you can add new categories in minor versions.

Problem: gyres-core now has opinions about domains (LLM, RL). And the LLM variant stores `serde_json::Value`, not a typed `Message` — you've lost the type safety you wanted.

### Approach 3: DomainTurn trait with type erasure at persistence boundary

This is closest to what I think you want:

```rust
// gyres-core: the persistence layer uses type-erased turns
pub struct SerializedTurn {
    pub timestamp: SystemTime,
    pub domain: String,
    pub observation: serde_json::Value,
    pub action: serde_json::Value,
    pub feedback: Option<serde_json::Value>,
}

// gyres-core: trait for domain-specific turns to implement
pub trait DomainTurn: Send + Clone {
    const DOMAIN: &'static str;
    
    fn serialize(&self) -> SerializedTurn;
    fn deserialize(turn: &SerializedTurn) -> Result<Self, GyreError>
    where
        Self: Sized;
}
```

```rust
// gyres-llm: fully typed
#[derive(Clone)]
pub struct LlmTurn {
    pub timestamp: SystemTime,
    pub observation: Message,       // typed!
    pub action: Message,            // typed!
    pub feedback: Option<Score>,    // typed!
}

impl DomainTurn for LlmTurn {
    const DOMAIN: &'static str = "llm";
    
    fn serialize(&self) -> SerializedTurn {
        SerializedTurn {
            timestamp: self.timestamp,
            domain: "llm".into(),
            observation: serde_json::to_value(&self.observation).unwrap(),
            action: serde_json::to_value(&self.action).unwrap(),
            feedback: self.feedback.as_ref().map(|f| serde_json::to_value(f).unwrap()),
        }
    }
    
    fn deserialize(turn: &SerializedTurn) -> Result<Self, GyreError> {
        if turn.domain != "llm" {
            return Err(GyreError::State("wrong domain".into()));
        }
        Ok(LlmTurn {
            timestamp: turn.timestamp,
            observation: serde_json::from_value(turn.observation.clone())
                .map_err(|e| GyreError::State(e.to_string()))?,
            action: serde_json::from_value(turn.action.clone())
                .map_err(|e| GyreError::State(e.to_string()))?,
            feedback: turn.feedback.as_ref()
                .map(|f| serde_json::from_value(f.clone()))
                .transpose()
                .map_err(|e| GyreError::State(e.to_string()))?,
        })
    }
}
```

```rust
// gyres-rl: also fully typed
#[derive(Clone)]
pub struct RlTurn {
    pub timestamp: SystemTime,
    pub observation: StateVector,
    pub action: DiscreteAction,
    pub feedback: Option<Reward>,
}

impl DomainTurn for RlTurn {
    const DOMAIN: &'static str = "rl";
    // ...
}
```

**How it flows:**

```
LlmTurn (typed, in gyres-llm)
    ↓ .serialize()
SerializedTurn (Value-based, in gyres-core)
    ↓ StateStore.save_session()
JSON file / SQLite (persistence)
    ↓ StateStore.load_session()
SerializedTurn
    ↓ LlmTurn::deserialize()
LlmTurn (typed again)
```

The Gyre works with `LlmTurn`. The StateStore works with `SerializedTurn`. The `DomainTurn` trait is the bridge. New domains (RL, robotics, whatever) implement `DomainTurn` for their own turn type without touching gyres-core.

This gives you:
- Full type safety in domain code
- Extensibility without modifying core enums
- `#[non_exhaustive]` isn't needed (no enum to extend)
- Clean serialization boundary
- The `domain` field enables heterogeneous session loading

---

## Updated decisions

| # | Decision | Status |
|---|---|---|
| 1E | `Agent` uses `&self` on step/feedback (enables concurrent/batched/speculative). `&mut self` only on reset. Agent is `Send + Sync`. Default `step_batch` calls step sequentially; RL overrides for GPU batching. | **New — single trait, not two** |
| 2A | No hooks, no middleware. Gyre minimal. | **Confirmed** |
| 3B | Strategy as associated type on Gyre: `type Strategy: Send + Sync`. Passed to `run()`. Gyre stays stateless. | **New** |
| 5B | `DomainTurn` trait + `SerializedTurn` for type-erased persistence. Domain crates implement typed turns with serialize/deserialize. | **Revised from enum proposal** |
| Gyre::run | `fn run(&self, agent: &A, ctx: &GyreContext, strategy: &Self::Strategy)` — all shared references | **Updated from 1E + 3B** |
