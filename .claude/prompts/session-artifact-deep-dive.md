# Session: Artifact Concept Deep Dive (gyres-b1h)

We're continuing work on **gyres** (`~/code/oss/gyres`), a Rust agent OS. Session 1 completed Phase 0 foundation decisions (ADRs 0014-0017: OS/harness seams, tokio, crate namespaces, versioning) and added Backend + MemoryStore + TaskStore + ArtifactStore traits to gyres-core.

## Context to load

Before starting, read these files to understand the current state:
1. `docs/src/vision/agent-os-vs-harness.md` — the 7 seams between harness and OS concerns (especially Memory/State and Persistence sections)
2. `docs/src/concepts/implicit-side-effects.md` — the vision for artifacts as implicit agent work products
3. `docs/src/concepts/store-abstraction.md` — how ArtifactStore fits in the store architecture
4. `crates/gyres-core/src/artifact.rs` — current ArtifactStore trait (emit/search/get/list)
5. `crates/gyres-core/src/memory.rs` — MemoryStore trait (store/recall/get/forget) — overlaps with artifacts
6. `crates/gyres-core/src/task.rs` — TaskStore trait — tasks may themselves be artifacts

Then run `bd show gyres-b1h` for the full issue description with all questions to resolve.

## The problem

"Artifact" is overloaded. The current ArtifactStore treats artifacts as implicit side-effects of agent work (decisions, documentation, code changes). But:

- **PM artifacts** (PRDs, ADRs, SPECs, ROADMAPs, feature specs) are also artifacts
- **FDD/BDD artifacts** (feature lists, BDD scenarios, acceptance criteria) are artifacts  
- **Tasks themselves** might be artifacts (an agent decomposes work and the decomposition is a work product)
- **Memory entries** overlap — when does an artifact become a memory? Are they the same thing with different access patterns?

We need a coherent concept that handles all of these without collapsing into "everything is an artifact."

## Questions to resolve (from the issue)

1. What is the taxonomy? Code artifacts vs document artifacts vs decision artifacts vs PM artifacts?
2. How do Artifact types relate to the PM hierarchy (`bd show gyres-wy3`, `bd show gyres-asn`)? Is a Task an artifact? Is an ADR?
3. Should Artifact have a type system (enum of known kinds) or stay stringly-typed (`kind: String`)?
4. What's the lifecycle? Created → stored → searchable → consolidated → archived?
5. How does RAG retrieval work in practice? What makes an artifact useful as future context?
6. Relationship to MemoryStore: where is the seam between artifacts and memories?
7. How does ArtifactStore make the "implicit side-effects" concept concrete?
8. Should artifacts be versioned? (Agent updates a decision — new artifact or update?)

## Related issues

- `gyres-wy3` — PM planning workflow (parent epic for task/PM concepts)
- `gyres-asn` — PM concept hierarchy survey (Theme→Epic→Story→Task) — overlaps heavily
- `gyres-vz2` — ArtifactStore GyreContext integration (blocked by b1h)
- `gyres-24p` — ADR/SPEC/DOC as Artifact vs Document trait (Phase 6 ideation, related)
- `gyres-3m1` — FDD/BDD document types and implicit side-effects

## How to work

- Use `bd update gyres-b1h --claim` when starting
- This is a **discussion and design** issue, not implementation. Output should be:
  - A concept doc in `docs/src/concepts/` (e.g., `artifact-taxonomy.md`)
  - Potentially an ADR if we make a load-bearing decision
  - Updates to `crates/gyres-core/src/artifact.rs` if the trait needs to change
- Consider whether `gyres-asn` (PM hierarchy) should be tackled first or in parallel — they inform each other
- Create ADRs via `adrs new "<title>" --no-edit` then edit the file (next number is 0018)
- Use `bd close gyres-b1h --reason "..."` when resolved
- Push git + `bd dolt push` when done

## What this unblocks

`gyres-vz2` (ArtifactStore GyreContext integration) is directly blocked by this. The artifact taxonomy also informs `gyres-k1g` (Task as first-class type) and the broader implicit side-effects vision.
