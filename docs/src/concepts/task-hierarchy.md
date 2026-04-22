# Task Hierarchy: PM Concepts in Gyres

> How standard project management hierarchy maps to gyres' Task type.
> Related: [artifact-taxonomy.md](artifact-taxonomy.md), gyres-asn, gyres-wy3.

## Design Decision

All levels of the PM hierarchy are modeled as **Task** with a `kind` field. There is no separate type for Theme, Epic, or Story — the graph structure and `kind` discriminator provide the hierarchy. This avoids duplicating DAG operations (blocking, dependency resolution, status rollup) across multiple types.

## The Standard PM Hierarchy

### Work Items (what)

These form a decomposition tree from strategic direction down to atomic actions:

| Level | What it is | Scope | Timeframe | Typical Owner |
|---|---|---|---|---|
| **Theme** | Organization-wide strategic direction. Not a deliverable — a compass heading. | Company-wide | Annual / multi-quarter | CPO, VP Product |
| **Initiative** | A cross-team program that advances a theme. The bridge between strategy and execution. | Multi-team | 1-2 quarters | Senior PM, Program Manager |
| **Epic** | A shippable capability for a single team. The primary unit of planning. | Single team | 2-8 weeks (1-3 sprints) | PM + Tech Lead |
| **Story** | A user-facing increment of value, completable in one sprint. The fundamental unit of Agile delivery. | 1-2 developers | 1-5 days | PM (writes), developer (implements) |
| **Task** | A concrete technical step to implement a story. Developer-facing, describes *how*. | Individual | Hours to 1-2 days | Individual developer |
| **Subtask** | An atomic action. The most granular trackable unit. | Individual | Minutes to hours | Individual developer |

### Time-boxes (when)

Sprints (Scrum) and Cycles (Linear) are **not work items**. They are time containers that work items are scheduled into. The relationship between work items and time-boxes is many-to-many:

- A sprint contains stories from multiple epics
- An epic's stories span multiple sprints
- A story is *assigned to* a sprint, not *child of* a sprint

```
WORK ITEM HIERARCHY (what)        TIME-BOX HIERARCHY (when)
─────────────────────────         ────────────────────────
Theme                             Program Increment (SAFe: 8-12 weeks)
  └─ Initiative                     └─ Sprint / Cycle (1-4 weeks)
       └─ Epic                           └─ Daily standup
            └─ Story
                 └─ Task
                      └─ Subtask
```

These hierarchies are orthogonal. Gyres models work items as Tasks in a DAG. Time-boxes, if needed, would be a separate concept (not a Task).

## Level Details

### Theme

A broad strategic area of focus. Themes are directional — they guide investment decisions but are not deliverable in themselves.

**Examples:**
- "Improve developer experience"
- "Expand into the European market"
- "AI-first platform"

**Fields beyond base Task:** success metrics (OKR-style), time horizon, strategic rationale.

**In gyres:** `Task { kind: "theme", ... }`. Children are Initiatives. Status is simple: Active / Retired. A Theme typically has no assignee — it's owned at the organizational level. In gyres, a Theme's `assignee` could be an orchestrator agent responsible for strategic planning.

### Initiative

A large, cross-cutting program that advances a Theme. Initiatives are time-bounded and may span multiple teams.

**Examples:**
- Theme "Improve DX" → Initiative "Redesign the CLI onboarding flow"
- Theme "Expand into EU" → Initiative "GDPR compliance program"

**Fields beyond base Task:** target quarter, owning team(s), linked theme, percent complete (rolled up from child epics).

**In gyres:** `Task { kind: "initiative", ... }`. Parent is a Theme (via parent edge in the DAG). Children are Epics. An initiative's status can be derived from its children's completion percentage.

### Epic

A shippable capability for a single team, decomposable into stories. Epics are the primary planning unit — large enough to be meaningful, small enough to deliver in weeks.

**Examples:**
- Initiative "Redesign CLI onboarding" → Epic "Implement interactive project scaffolding wizard"
- Initiative "GDPR compliance" → Epic "Build data export pipeline"

**Common decomposition strategies:**
- By workflow step
- By business rule or data variation
- By interface (API, UI, CLI)
- By CRUD operation

**Fields beyond base Task:** acceptance criteria (high-level), target dates, story point rollup.

**In gyres:** `Task { kind: "epic", ... }`. Parent is an Initiative. Children are Stories. The DAG's `blockers()`/`dependents()` with depth traversal naturally models the epic-to-story decomposition.

### Story (User Story)

The fundamental unit of Agile delivery. Describes user-facing value in a standard format, sized to fit within a single sprint.

**Standard format (Connextra):**
```
As a [type of user],
I want [some goal/action],
So that [some reason/benefit].
```

**Acceptance criteria (Gherkin/BDD):**
```
Given I am on the login page
When I enter valid credentials and click "Sign In"
Then I am redirected to the dashboard
```

**Sizing:** Story points (Fibonacci: 1, 2, 3, 5, 8, 13). Stories above 8 points should be split.

**Fields beyond base Task:** story points, acceptance criteria, sprint/cycle assignment.

**In gyres:** `Task { kind: "story", ... }`. Parent is an Epic. Children are Tasks. Acceptance criteria and story points live in `metadata`. Sprint assignment is a relationship to a time-box, not a DAG edge — modeled via `relate()` or metadata.

### Task

A concrete technical work item. Developer-facing, describes *how* to build something rather than *what* the user gets. Tasks can exist without a parent story (for technical debt, spikes, infrastructure work).

**Examples:**
- Story "Export data as CSV" → Tasks:
  - "Add CSV serialization to the export module"
  - "Create REST endpoint GET /api/export?format=csv"
  - "Write integration tests for CSV export"

**Fields beyond base Task:** estimated hours, branch/PR link.

**In gyres:** `Task { kind: "task", ... }`. This is the default kind — when `kind` is not specified, it's a task. Parent is typically a Story, but can be an Epic directly for non-story work.

### Subtask

The most granular unit. Atomic actions that decompose a task. Not all tasks need subtasks — they exist for tracking fine-grained progress.

**Examples:**
- Task "Add CSV serialization" → Subtasks:
  - "Define CsvRow struct"
  - "Implement Display trait for CsvRow"
  - "Handle special character escaping"

**In gyres:** `Task { kind: "subtask", ... }`. Parent is a Task. Leaf node in the hierarchy.

## The Gyres Task Model

All levels share the same `Task` struct and live in `TaskStore`. The `kind` field discriminates:

```rust
pub enum TaskKind {
    Theme,
    Initiative,
    Epic,
    Story,
    Task,
    Subtask,
    Custom(String),  // telemetry-logged to signal new kinds needed
}
```

Like `DocumentKind`, `Custom(String)` usage is telemetry-logged to signal when new kinds should be promoted to first-class variants.

### Hierarchy as DAG Edges

The parent-child hierarchy is modeled via TaskStore's existing relationships:

```
Theme A ──parent-of──→ Initiative B
Initiative B ──parent-of──→ Epic C
Epic C ──parent-of──→ Story D
Story D ──parent-of──→ Task E
Task E ──parent-of──→ Subtask F
```

`TaskStore::blockers()` and `dependents()` with depth traversal navigate the tree. `ready_tasks()` finds leaf-level work that can begin.

### Optional Parents

The hierarchy is not rigid. Common patterns:

- **Technical debt task:** `Task { kind: "task" }` with no parent story — lives under an Epic directly
- **Spike:** `Task { kind: "story" }` with no parent epic — standalone investigation
- **Standalone bug:** `Task { kind: "task" }` with no parent at all
- **Flat backlog:** All items are `Task { kind: "task" }` — no hierarchy used

The model supports any depth from flat (no hierarchy) to deep (6+ levels).

### Status Transitions Per Kind

Different levels have different valid status workflows:

| Kind | Typical statuses |
|---|---|
| Theme | Active, Retired |
| Initiative | Proposed, Active, Complete, Cancelled |
| Epic | To Do, In Progress, Done, Cancelled |
| Story | To Do, In Progress, In Review, Done, Cancelled |
| Task | Blocked, Ready, In Progress, Complete, Failed, Cancelled |
| Subtask | To Do, Done |

The current `TaskStatus` enum (Blocked, Ready, InProgress, Complete, Failed, Cancelled) covers the most granular case. Higher levels may use a subset. Whether to enforce per-kind status validation or keep a single permissive enum is a future implementation decision.

## External Tool Mapping

Different PM tools model the hierarchy differently. Gyres' canonical `TaskKind` maps to each:

| Gyres TaskKind | Jira | Linear | Asana | GitHub Projects |
|---|---|---|---|---|
| Theme | Custom level (Advanced Roadmaps) | Label / external | Portfolio or Goal | Label / external |
| Initiative | Custom level or labeled Epic | Project | Project or Portfolio | Project board |
| Epic | Epic (built-in) | Project or parent Issue | Project or Milestone | Tracking issue or Milestone |
| Story | Story (built-in) | Issue | Task | Issue |
| Task | Task or Sub-task (built-in) | Sub-issue | Subtask | Checklist item or sub-Issue |
| Subtask | Sub-task (built-in) | Nested sub-issue | Sub-subtask | Checklist item |

Future adapter crates (`gyres-sync`) will map between gyres Tasks and external platforms. The seam:
- `TaskKind` maps to the platform's hierarchy level
- `Task.metadata` carries platform-specific fields (Jira issue key, story points, sprint ID)
- Future `external_refs` field provides bidirectional ID mapping

## Development Methodology Mappings

Each development methodology produces its own artifacts and decomposes work differently. This section maps their concepts to gyres primitives (Tasks, Documents, Memories) and identifies what the Gyre could capture as implicit side-effects.

### How Methodologies Complement Each Other

These methodologies are not alternatives — they operate at different levels and combine:

```
DDD  ─── architecture & domain modeling ──── "build the right structure"
FDD  ─── delivery planning & tracking  ──── "deliver predictably"
BDD  ─── behavioral specification      ──── "build what users need"
ATDD ─── acceptance criteria alignment  ──── "agree on done"
TDD  ─── code-level correctness        ──── "build it correctly"
SDD  ─── spec-first AI development     ──── "specify for AI agents"
```

DDD provides the architecture. BDD/ATDD define what "right" means from the user/business perspective. TDD ensures the code is correct. FDD provides delivery discipline. SDD adapts the spec-first approach for AI-assisted development — directly relevant to how gyres agents work.

### DDD (Domain-Driven Design)

**Focus:** Complex business logic and domain modeling. Aligns software structure with business capabilities.

**Approach:** Build a ubiquitous language with domain experts, identify bounded contexts, apply tactical patterns (entities, value objects, aggregates, domain events).

**Artifacts and gyres mapping:**

| DDD Artifact | Gyres Primitive | Notes |
|---|---|---|
| Bounded Context | Task (kind: theme or initiative) | A bounded context defines a domain boundary — maps to a high-level organizational grouping |
| Domain Model | Document (kind: spec or custom "domain-model") | The model is a structured specification of entities, relationships, and rules |
| Ubiquitous Language glossary | Document (kind: custom "glossary") or Memory | Terms and definitions shared across the team. As a Document: formal, versioned. As Memory: agent-recallable context for consistent language use |
| Context Map | Document (kind: custom "context-map") | Diagram/description of how bounded contexts relate (published language, shared kernel, anti-corruption layer) |
| Domain Events | Implicit side-effect | When an agent detects a state change in the domain, the Gyre can capture it as a domain event record |
| Aggregates, Entities, Value Objects | Code artifacts (not gyres primitives) | These are implementation patterns — they live in code, not in the task/artifact system |

**Implicit side-effects for gyres:** When an agent is working within a bounded context, the Gyre can automatically maintain the ubiquitous language glossary (MemoryStore) and detect when domain events are being defined (DocumentStore).

### FDD (Feature Driven Development)

**Focus:** Structured agile delivery of client-valued features in short, predictable iterations with visible progress tracking.

**Workflow:** Develop overall model → Build feature list → Plan by feature → Design by feature → Build by feature.

**Artifacts and gyres mapping:**

| FDD Artifact | Gyres Primitive | Notes |
|---|---|---|
| Subject Area | Task (kind: theme) | Broad domain grouping |
| Business Activity | Task (kind: epic) | Decomposable unit of business value |
| Feature | Task (kind: story) | Completable in 2 weeks or less. FDD format: "\<action\> the \<result\> \<by/for/of/to\> a(n) \<object\>" |
| Feature List | Document (kind: feature-spec or plan) | The organized list of all features, grouped by subject area and business activity |
| Design Package | Document (kind: spec) | Per-feature design produced during "Design by Feature" |
| Overall Model | Document (kind: spec) | The domain model produced in step 1 |
| Progress report | Telemetry / derived from TaskStore | Feature completion percentages — derived from task status rollup |

FDD features are equivalent in scope to User Stories, not to SAFe "features" (which are closer to Epics).

**Implicit side-effects for gyres:** When an agent decomposes work, the Gyre can automatically produce the feature list (Document) and design packages (Document) as side-effects of the planning process.

### BDD (Behavior-Driven Development)

**Focus:** User behavior and system functionality from the user's perspective. Fosters collaboration between non-technical stakeholders, developers, and testers.

**Approach:** Discover behaviors with stakeholders → Describe scenarios in Given/When/Then (Gherkin) → Automate and implement.

**Artifacts and gyres mapping:**

| BDD Artifact | Gyres Primitive | Notes |
|---|---|---|
| Feature file (.feature) | Document (kind: feature-spec) | Structured Gherkin document describing a feature's behavior |
| Scenario (Given/When/Then) | Embedded in feature-spec content, or Task (kind: story) | Each scenario is a concrete example of behavior. As a story: independently implementable and testable |
| Scenario Outline (with Examples) | Embedded in feature-spec content | Parameterized scenarios — part of the feature spec document |
| Living documentation | ArtifactStore query result | The set of all feature-specs IS the living documentation — queryable via ArtifactStore search |
| Step definitions | Code artifacts (not gyres primitives) | Test glue code — lives in the codebase |

**BDD scenario format:**
```gherkin
Feature: User authentication
  Scenario: Successful login
    Given a registered user with email "user@example.com"
    When they submit valid credentials
    Then they are redirected to the dashboard
    And they see a welcome message
```

**Implicit side-effects for gyres:** When an agent writes acceptance criteria for a story, the Gyre can detect Given/When/Then patterns and automatically emit a feature-spec Document. The collection of all feature-specs becomes living documentation searchable via ArtifactStore.

### ATDD (Acceptance Test-Driven Development)

**Focus:** Collaborative definition of acceptance criteria before coding. Ensures all stakeholders agree on requirements.

**Approach:** Three Amigos (business, dev, test) define acceptance examples → Automate acceptance checks → Code until checks pass.

**Artifacts and gyres mapping:**

| ATDD Artifact | Gyres Primitive | Notes |
|---|---|---|
| Acceptance criteria | Metadata on Task (kind: story) | "Given/When/Then" or bullet-list conditions stored in story metadata or description |
| Acceptance tests | Code artifacts + reference in Task metadata | Automated tests that verify criteria — code lives in repo, task links to them |
| Three Amigos session output | Document (kind: custom "requirements") or Memory | The collaborative discussion output — requirements decisions worth preserving |
| Requirement specifications | Document (kind: prd or spec) | Formalized requirements derived from acceptance discussions |

**Relationship to BDD:** ATDD and BDD overlap significantly. BDD adds the Given/When/Then scenario language and emphasizes behavioral description. ATDD emphasizes the collaborative definition process. In practice, BDD is often "ATDD plus a scenario style."

**Implicit side-effects for gyres:** When an agent defines acceptance criteria on a story, the Gyre can verify they are testable and flag stories without acceptance criteria. The "Three Amigos" pattern maps to multi-agent collaboration — a planner agent, implementer agent, and reviewer agent aligning on acceptance.

### TDD (Test-Driven Development)

**Focus:** Code-level correctness through the Red-Green-Refactor cycle.

**Approach:** Write a failing test → Write minimal code to pass → Refactor while tests guard behavior.

**Artifacts and gyres mapping:**

| TDD Artifact | Gyres Primitive | Notes |
|---|---|---|
| Unit tests | Code artifacts (not gyres primitives) | Tests live in the codebase — they are code, not task/artifact system items |
| Test suite | Code artifact | The collection of all tests |
| Refactoring decisions | Memory or implicit side-effect | "Refactored X because Y" — the rationale is worth capturing as Memory or a lightweight decision artifact |
| Red-Green-Refactor cycle | Gyre loop pattern | The TDD cycle maps directly to a Gyre's feedback loop: observe (test fails) → act (write code) → feedback (test passes) → refactor |

**TDD is mostly code, not artifacts.** Unlike the other methodologies, TDD produces primarily code-level artifacts (tests). The gyres mapping is more about the *process* than the *work products*:

- The Red-Green-Refactor cycle is a natural Gyre feedback loop
- An agent doing TDD would: write a test (Action) → run it (Observe failure) → write implementation (Action) → run it (Observe success) → refactor (Action)
- The Gyre captures refactoring rationale as Memory

### SDD (Spec-Driven Development)

**Focus:** Specification-first development for AI-assisted workflows. The spec is the source of truth for both humans and AI agents.

**Approach:** Write a structured, behavior-oriented specification → AI generates code from the spec → Humans review and iterate on the spec, not the code.

**Three maturity levels:**
1. **Spec-first** — specification precedes AI-assisted development for a task
2. **Spec-anchored** — specification persists and evolves with feature maintenance
3. **Spec-as-source** — specification is the primary artifact; humans edit specs, never code

**Artifacts and gyres mapping:**

| SDD Artifact | Gyres Primitive | Notes |
|---|---|---|
| Specification | Document (kind: spec) | The primary work product — structured, behavior-oriented, natural language |
| Requirements (user stories + acceptance criteria) | Task (kind: story) + Document (kind: spec) | Some SDD tools (Kiro) decompose specs into stories with acceptance criteria |
| Design document | Document (kind: plan) | Intermediate design produced from the spec before implementation |
| Task decomposition | Task (kind: task) | Implementation tasks derived from the spec |
| Constitution / rules | Document (kind: custom "constitution") or Memory | System-wide rules that apply across all specs (Spec-kit pattern) |

**SDD is directly relevant to gyres.** An agent operating in SDD mode would:
1. Receive or generate a spec (Document)
2. Decompose it into tasks (TaskStore)
3. Implement against the spec, with the spec as RAG context (ArtifactStore search)
4. Update the spec as the implementation evolves (DocumentStore versioning)

The spec-anchored and spec-as-source levels map to gyres' Document versioning model — lifecycle transitions create snapshots, edits within a state are mutable.

**Caution from Martin Fowler's analysis:** SDD at the "spec-as-source" level inherits risks from Model-Driven Development — the spec may become too rigid or diverge from what the code actually does, compounded by LLM non-determinism. Gyres should support SDD at all three levels but not mandate spec-as-source.

## Methodology Artifacts Summary

How each methodology's artifacts map to gyres primitives:

| Artifact Type | Methodology Source | Gyres Primitive | Store |
|---|---|---|---|
| Feature list | FDD | Document (plan) | DocumentStore |
| Feature file / scenario | BDD | Document (feature-spec) | DocumentStore |
| Acceptance criteria | ATDD / BDD | Task metadata (story) | TaskStore |
| Domain model | DDD | Document (spec) | DocumentStore |
| Ubiquitous language | DDD | Memory or Document | MemoryStore or DocumentStore |
| Context map | DDD | Document | DocumentStore |
| Specification | SDD | Document (spec) | DocumentStore |
| Design package | FDD | Document (spec) | DocumentStore |
| Unit tests | TDD | Code (not a gyres primitive) | — |
| Refactoring rationale | TDD | Memory | MemoryStore |
| Living documentation | BDD | ArtifactStore query | ArtifactStore (RefStore) |
| Progress tracking | FDD | Derived from TaskStore | TaskStore |

## Document Types in the Development Lifecycle

The Document kinds defined in [artifact-taxonomy.md](artifact-taxonomy.md) (ADR, PRD, Roadmap, Plan, FeatureSpec) map to specific roles in the development lifecycle. Different methodologies produce different documents at different times.

### Document Kind Reference

| Document Kind | What it is | When produced | Methodology source | Typical lifecycle |
|---|---|---|---|---|
| **PRD** | Product Requirements Document. Business-facing description of what to build and why. User needs, market context, success metrics. | Before development begins. Initiative/Epic level. | General PM practice | Draft → Review → Approved → Amended |
| **SPEC** | Formal specification. Behavior-oriented requirements using SHALL/MUST language and GIVEN/WHEN/THEN scenarios. | Before or during development. Epic/Story level. | SDD, BDD, ATDD, DDD | Draft → Proposed → Accepted → Superseded |
| **ADR** | Architecture Decision Record. Captures a specific architectural choice with context, alternatives considered, and consequences. | During development when load-bearing decisions are made. | DDD (architectural), general practice | Proposed → Accepted → Deprecated → Superseded |
| **Roadmap** | Strategic planning document. Maps initiatives and epics to time horizons. | Planning phase. Theme/Initiative level. | FDD (feature list), SAFe (PI planning) | Draft → Active → Archived |
| **Plan** | Implementation plan. Technical approach, task decomposition, architecture decisions for a specific feature or change. | After spec, before implementation. Epic/Story level. | SDD, FDD (design by feature) | Draft → Active → Complete |
| **FeatureSpec** | Feature behavior specification. BDD feature files, acceptance criteria, Given/When/Then scenarios. | During story refinement. Story level. | BDD, ATDD, FDD | Draft → Accepted → Implemented |

### How Documents Flow Through Development

```
Strategic planning:     PRD ──→ Roadmap
                                  │
Decomposition:                    ├──→ SPEC (per epic/feature)
                                  │      │
Architecture:                     │      ├──→ ADR (when decisions are made)
                                  │      │
Implementation:                   │      ├──→ Plan (technical approach)
                                  │      │      │
Story refinement:                 │      │      ├──→ FeatureSpec (BDD scenarios)
                                  │      │      │
Code:                             │      │      └──→ Tasks → Implementation
                                  │      │
Feedback:                         │      └──→ SPEC updated (delta specs)
                                  │
Closure:                          └──→ Roadmap updated, ADRs finalized
```

Documents are versioned via the hybrid model (edits within a state are mutable, lifecycle transitions create immutable snapshots). This means a SPEC evolves during development but its transition from Proposed → Accepted is captured as an immutable record.

### Agent Toolkit Comparison

Several AI-assisted development tools implement SDD-like workflows. Each produces a specific set of documents. Understanding their patterns informs what gyres' DocumentStore should support.

#### Kiro (AWS)

**Workflow:** Three-phase spec-driven development.

**Directory structure:**
```
.kiro/specs/[feature-name]/
├��─ requirements.md     # User stories + acceptance criteria (or bugfix.md for bugs)
├��─ design.md           # System architecture, sequence diagrams, error handling
└── tasks.md            # Discrete implementation tasks with status tracking
```

**Gyres mapping:**

| Kiro Artifact | Gyres Document Kind | Notes |
|---|---|---|
| requirements.md | FeatureSpec or PRD | User stories with acceptance criteria |
| bugfix.md | FeatureSpec (or custom "bugfix") | Current vs expected behavior analysis |
| design.md | Plan | Technical architecture and component design |
| tasks.md | Tasks in TaskStore | Implementation breakdown with real-time status |

#### Spec-kit

**Workflow:** Six-step SDD — Constitution → Specify → Clarify → Plan → Analyze → Tasks → Implement.

**Directory structure:**
```
specs/[###-feature-name]/
├── spec.md             # User stories & acceptance criteria (prioritized P1/P2/P3)
├── plan.md             # Technical implementation plan with architecture decisions
├── research.md         # Research phase output
├── data-model.md       # Data entities & relationships
├── quickstart.md       # Key validation scenarios
└── contracts/          # API endpoint definitions (YAML/JSON)

constitution.md          # Project-wide governing principles (lives at project root)
```

**Gyres mapping:**

| Spec-kit Artifact | Gyres Document Kind | Notes |
|---|---|---|
| constitution.md | Document (custom "constitution") or Memory | Project-wide rules and constraints. As Memory: always-recalled agent context. As Document: versioned governance rules |
| spec.md | SPEC | Prioritized user stories with acceptance criteria and success metrics |
| plan.md | Plan | Technical architecture, stack decisions, complexity justifications |
| research.md | Memory or Document (custom "research") | Research findings — Memory if for agent recall, Document if for human reference |
| data-model.md | Document (custom "data-model") or SPEC | Entity definitions and relationships |
| contracts/ | Document (custom "api-contract") or SPEC | API endpoint definitions |
| tasks.md | Tasks in TaskStore | Organized by user story priority, with parallelization markers |

**Notable concept:** The **constitution** is a project-wide document that applies across all features — similar to how gyres' MemoryStore carries persistent agent knowledge. In gyres, a constitution could be both a Document (formal, versioned) and loaded into MemoryStore (always available as agent context).

#### OpenSpec

**Workflow:** Propose → Explore → Apply → Verify → Archive. Fluid, no phase gates.

**Directory structure:**
```
openspec/
├── specs/                      # Source of truth (current behavior contracts)
│   ���── auth/
��   │   └── spec.md             # Behavior contract with SHALL/MUST requirements
│   └── payments/
│       └── spec.md
├── changes/                    # Active change proposals
│   └── add-dark-mode/
│       ├── proposal.md         # Intent, scope, approach (the "why")
│       ├─�� design.md           # Technical approach (the "how")
│       ├��─ tasks.md            # Implementation checklist (the "steps")
│       └── specs/              # Delta specs (ADDED/MODIFIED/REMOVED requirements)
└── changes/archive/            # Completed changes (history)
    └── 2025-01-24-add-dark-mode/
```

**Gyres mapping:**

| OpenSpec Artifact | Gyres Document Kind | Notes |
|---|---|---|
| specs/\*/spec.md | SPEC | Behavior contracts using RFC 2119 keywords (SHALL/MUST/SHOULD/MAY) with GIVEN/WHEN/THEN scenarios |
| proposal.md | Document (custom "proposal") or PRD | Intent and scope for a change |
| design.md | Plan | Technical architecture decisions |
| tasks.md | Tasks in TaskStore | Implementation checklist |
| Delta specs | SPEC (new version via DocumentStore versioning) | ADDED/MODIFIED/REMOVED requirements — maps directly to gyres' hybrid versioning model |
| Archive | DocumentStore version history | Completed changes become version snapshots |

**Notable concepts:**
- **Delta specs** — changes are expressed as diffs against current behavior, not full rewrites. This maps perfectly to gyres' Document versioning: the delta is the edit, archiving is a lifecycle transition that creates a snapshot.
- **Brownfield-first** — OpenSpec assumes existing systems. Most specs describe changes to existing behavior, not greenfield requirements. Gyres' ArtifactStore cross-type search supports this by providing prior spec context when writing new changes.
- **Fluid lifecycle** — OpenSpec has no phase gates; any artifact can be updated at any time. This aligns with gyres' "edits within a state are mutable" model.

#### Tessl

**Workflow:** Create → Evaluate → Distribute agent skills (packaged context).

Tessl takes a different approach from the other tools — rather than producing specs-per-feature, it produces **skills**: versioned, packaged units of context that AI agents consume. Skills contain API documentation, usage patterns, library integration guidance, and organizational conventions.

| Tessl Artifact | Gyres Primitive | Notes |
|---|---|---|
| Skill | Memory (persistent agent context) | Versioned knowledge packages — maps to MemoryStore entries that agents recall when working in a specific domain |
| Evaluation results | Telemetry | Skill performance metrics |

**Notable concept:** Tessl's skills are essentially curated, versioned MemoryStore entries. The "create → evaluate → distribute" lifecycle maps to: store Memory → validate via Gyre feedback loop → share across agents via MemoryStore cross-agent access.

### Agent Toolkit Document Flow Comparison

How each toolkit's workflow maps to gyres stores:

```
                    Kiro            Spec-kit         OpenSpec          Tessl
                    ────            ────────         ────────          ─────
Governance          —               constitution.md  —                 skills
                                    ↓ Memory/Doc                      ↓ Memory

Requirements        requirements.md spec.md          proposal.md       —
                    ↓ FeatureSpec   ↓ SPEC           + delta specs
                                                     ↓ SPEC/PRD

Design              design.md       plan.md          design.md         —
                    ↓ Plan          + data-model.md  ↓ Plan
                                    + contracts/
                                    ↓ Plan/SPEC

Tasks               tasks.md        tasks.md         tasks.md          —
                    ↓ TaskStore     ↓ TaskStore      ↓ TaskStore

Archive             —               —                changes/archive/  —
                                                     ↓ Doc versions
```

### Implications for Gyres DocumentStore

The agent toolkit survey reveals patterns gyres should support:

1. **The three-doc pattern is universal.** Every toolkit produces some form of requirements → design → tasks. Gyres' Document kinds (SPEC/FeatureSpec, Plan, and Tasks in TaskStore) already cover this.

2. **Project-wide governance documents exist.** Spec-kit's constitution and Tessl's skills are project-scoped, not feature-scoped. These map to either long-lived Documents or persistent Memory entries. Gyres should support both patterns.

3. **Delta specs are the norm for brownfield work.** OpenSpec's ADDED/MODIFIED/REMOVED pattern maps directly to gyres' Document versioning model. The hybrid versioning (mutable edits + immutable snapshots on transitions) handles this naturally.

4. **All artifacts are Markdown.** Every toolkit uses Markdown files. Gyres' Document `path: Option<PathBuf>` field (linking Documents to files on disk) aligns perfectly.

5. **Tasks are always the leaf output.** Every toolkit ends with a tasks.md that decomposes into implementation work. This confirms TaskStore as the terminal store in the planning pipeline, with Documents feeding into task decomposition.

## Framework Comparison

### SAFe (Scaled Agile Framework)

The most elaborate hierarchy, designed for large enterprises:

```
PORTFOLIO LEVEL
  Strategic Theme
    └─ Epic (Portfolio Epic)          — large cross-cutting initiative
         └─ Capability                — solution-level feature

PROGRAM LEVEL (Agile Release Train)
  Program Increment (PI)              — time-box: 8-12 weeks
    └─ Feature                        — team-level deliverable
         └─ Story                     — sprint-level work item

TEAM LEVEL
  Iteration (Sprint)                  — time-box: 2 weeks
    └─ Story → Task
```

SAFe distinguishes Portfolio Epics (massive, funded programs), Features (team-level deliverables within a Program Increment), and Capabilities (cross-train features). Gyres maps these to Initiative, Epic, and Epic respectively — the distinction is organizational scope, not structural.

### Scrum (standard)

Scrum defines very little hierarchy:

```
Product Backlog
  └─ Product Backlog Item (PBI)       — usually a User Story
       └─ Task

Sprint (time-box)
  └─ Sprint Backlog (subset of PBIs)
```

Scrum does not define Themes, Initiatives, or Epics in the Scrum Guide. These are additions from the broader Agile community.

### Kanban

Kanban prescribes no hierarchy or time-boxing. Work items flow through columns (states) with WIP limits. Teams often adopt a light hierarchy (epic → story → task) by convention, but this is not part of the method.

## Agile Concepts as Gyres Primitives

Not every Agile concept is a stored type. Some are query patterns, some are computed views, some are relationship patterns. This section classifies each.

### Classification Table

| Agile Concept | Gyres Primitive | Classification | Notes |
|---|---|---|---|
| Sprint / Cycle | Separate type (not a Task) | Time-box | Many-to-many relationship to Tasks via `relate()` or metadata. See "Time-boxes" section above. |
| Product Backlog | `TaskStore::list_tasks()` query | Query pattern | All open tasks sorted by priority. Not a stored entity. |
| Sprint Backlog | `TaskStore::list_tasks()` + sprint filter | Query pattern | Tasks assigned to a specific sprint (time-box). A filtered subset of the product backlog. |
| PRD | Document (kind: prd) | Stored artifact | See [artifact-taxonomy.md](artifact-taxonomy.md). |
| Roadmap | Document (kind: roadmap) | Stored artifact | See [artifact-taxonomy.md](artifact-taxonomy.md). |
| RACI Chart | Agent roles on Tasks | Relationship pattern | See detail below. |
| GANTT Chart | Computed from TaskStore DAG | Computed view | See detail below. |
| Burndown / Velocity | Computed from TaskStore + TelemetrySink | Computed view | Derived from task completion rates over time. Telemetry concern, not a store concern. |
| Retrospective | Document (kind: custom "retro") or Memory | Stored artifact | Lessons learned — Document if formal, Memory if agent-recallable knowledge. |
| Definition of Done | Memory or Document (kind: custom "dod") | Stored artifact | Team-wide criteria. As Memory: always-recalled agent context. As Document: versioned governance. Similar to spec-kit's constitution. |

### Backlogs

A backlog is a prioritized query over TaskStore, not a stored entity.

```rust
// Product backlog: all open work, sorted by priority
task_store.list_tasks(&TaskFilter {
    status: Some(TaskStatus::Ready),  // or {Ready, Blocked}
    ..Default::default()
})
// → sorted by Task.metadata["priority"]

// Sprint backlog: tasks assigned to a specific sprint
// (sprint assignment is a relationship or metadata field)
task_store.list_tasks(&TaskFilter {
    status: Some(TaskStatus::InProgress),
    ..Default::default()
})
// → filtered by Task.metadata["sprint_id"] or relate() edge

// Team backlog: tasks for a specific agent/team
task_store.list_tasks(&TaskFilter {
    assignee: Some(agent_id),
    status: Some(TaskStatus::Ready),
    ..Default::default()
})
```

Different backlog views are just different filter+sort combinations on the same TaskStore data.

### RACI (Responsible, Accountable, Consulted, Informed)

RACI maps agent roles to tasks. In single-agent gyres, only Responsible matters. In multi-agent, all four become relevant.

| RACI Role | Gyres Mapping | When it matters |
|---|---|---|
| **Responsible** | `Task.assignee: Option<AgentId>` | Always — the agent doing the work |
| **Accountable** | Parent task's assignee, or a dedicated `owner` field on Task | Multi-agent — the supervisor/orchestrator that delegated and owns the outcome |
| **Consulted** | `relate(task, "consults", agent_id)` or `Task.metadata["consulted"]` | Multi-agent — agents whose knowledge is needed before/during work |
| **Informed** | Subscription/notification pattern (MessageBus or TelemetrySink) | Multi-agent OS concern — agents notified of status changes |

**Design implication:** R is a first-class field (assignee, already on Task). A may warrant a second field (owner/accountable) if multi-agent delegation is common — or it can be derived from the parent task's assignee in the DAG. C and I are relationship/notification concerns, not Task fields.

### GANTT Charts

A GANTT chart is a computed view, not a stored artifact. It is derived from:

1. **TaskStore DAG** — dependency relationships (what blocks what)
2. **Time estimates** — `Task.metadata["estimate"]` (hours or story points)
3. **Start dates** — `Task.metadata["start_date"]` or derived from dependency resolution
4. **Critical path** — computed via `blockers()`/`dependents()` traversal

```
GANTT = TaskStore.list_tasks()
      + dependency graph (blockers/dependents traversal)
      + time estimates (metadata)
      + scheduling algorithm (earliest start, critical path)
      → computed schedule
      → rendered as chart (UI/document concern)
```

If an agent generates a GANTT chart as an output, the rendered chart is a Document (kind: plan or custom "gantt"). But the chart data is always derived from TaskStore — it is never the source of truth.

## Key Modeling Principles

1. **One type, many kinds.** All work items are Tasks. The `kind` field provides the hierarchy level. This avoids type proliferation and lets TaskStore's DAG operations work uniformly.

2. **Hierarchy is optional.** Not every team uses all levels. A solo developer might use flat tasks. An enterprise might use all six levels. The model supports both.

3. **Time-boxes are separate.** Sprints/Cycles are not Tasks. They are an orthogonal scheduling concept with a many-to-many relationship to Tasks.

4. **Not everything is stored.** Backlogs are queries. GANTT charts are computed views. Burndowns are telemetry. Only work items, documents, and memories are stored primitives. Resist the urge to reify every PM concept as a type.

5. **Status varies by kind.** Higher levels have simpler lifecycles. Per-kind validation can be layered on via the same `Lifecycle` trait pattern used for Documents, or kept permissive with a single enum.

6. **External tools differ.** The canonical hierarchy is a superset. Adapters map the subset each tool supports.
