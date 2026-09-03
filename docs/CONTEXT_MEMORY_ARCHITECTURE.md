# AIP Context & Memory Architecture

Status: **NORMATIVE ARCHITECTURE TARGET — NOT A CLAIM OF CURRENT IMPLEMENTATION**

This document is the canonical design authority for AIP context, memory, conversation-history, profile-projection, retrieval, compaction, and context-window management.

When older aspirational documentation conflicts with this document inside the context/memory domain, this document takes precedence. Historical validation records and statements about what a released or already-implemented build actually does remain factual evidence and must not be rewritten as though the target architecture already exists. Security, privacy, ownership, data-integrity, and Rust-authority invariants remain hard constraints. If a future implementation discovers a genuinely irreconcilable conflict with one of those hard constraints, the conflict must be surfaced explicitly rather than silently substituting behavior.

## 1. Foundation

AIP must never equate these concepts:

```text
full conversation history
!= long-term memory
!= active model context
```

The model context is a temporary compilation. Persistent knowledge lives outside the active model window.

The LLM may interpret content, extract candidate facts, classify durability, propose compaction, estimate future utility, identify relationships, and propose summaries. The LLM does not own persistence.

The deterministic AIP core owns:

- stable IDs and ownership;
- namespaces and categories;
- persistence and deletion;
- conflict resolution and supersession;
- expiration and archival;
- token budgets and output reservation;
- retrieval eligibility and context selection;
- auditability and provenance.

Rust/SQLite remain authoritative. Model output is candidate data until validated by deterministic policy.

## 2. Context layers

AIP must model context as separate layers:

```text
AIP KNOWLEDGE SYSTEM
|
+-- Agent Identity
+-- User Profile
+-- Agent State
+-- User State
+-- Long-Term Memory
+-- Episodic Memory
+-- Historical / Low-Priority Memory
+-- Temporary Context
+-- Conversation History
+-- Conversation Summaries
+-- Relationships / Opinions / Goals
+-- Runtime Context
     |
     v
 Context Compiler
     |
     v
 Token Budget
     |
     v
    LLM
```

The LLM receives only the bounded compiled result, never the complete database by default.

### Agent identity

Protected identity is high-priority authoritative state. It must not be reconstructed from conversation summaries.

### User profile

The User Profile is a structured projection of durable or semi-stable Owner knowledge. It is not a transcript and should preferably be rebuilt from authoritative atomic memories, current state, confirmed preferences, and important events.

### Agent and user namespaces

User knowledge and agent knowledge must remain independent namespaces. Information learned by one agent must not automatically become another agent's memory. Temporary chat must not silently populate durable profile knowledge.

## 3. Semantic memory, not transcript memory

Long-term memory must preserve meaning rather than wording.

Bad:

```text
The Owner said many sentences about replacing one device, considering alternatives,
checking temperatures, changing a setting, and reacting to benchmark results...
```

Better:

```yaml
hardware_upgrade:
  previous: example_device_a
  current: example_device_b
  maintenance:
    - cleaning
    - thermal_interface_service
  outcome: thermals_improved
```

Durable memory candidates must pass through:

```text
conversation
  -> candidate extraction
  -> durability classification
  -> relevance / future-utility classification
  -> compaction
  -> duplicate / update / conflict detection
  -> deterministic policy gate
  -> structured persistence
```

Compaction removes filler, repetition, exact phrasing, abandoned alternatives, redundant chronology, intermediate reasoning, and details with no plausible future utility. It preserves final state, meaningful previous state, important decisions, useful causal information, provenance, and dates when temporal meaning matters.

## 4. Durability and retention

Every candidate must receive a durability class.

### Ephemeral

Useful only for the current interaction. It remains in conversation/runtime context and is not durable memory.

### Short-lived

Useful for hours or days. It may be stored as first-class temporary context with a TTL or explicit completion condition.

### Medium-term

Useful for an ongoing task or project. When the task ends, AIP extracts the useful final result and then downgrades, archives, compacts, or deletes the working details.

### Durable

Likely to matter for months or years. This belongs in structured long-term memory.

### Historical

No longer current but plausibly useful for comparisons, chronology, compatibility, diagnostics, or shared history. Historical memory should normally be aggressively compressed and ranked below current state.

### Disposable

No meaningful expected future value. Discard it unless the rejection itself establishes an important preference or decision.

The retention decision is not binary. AIP uses:

```text
DELETE
COMPRESS
PRESERVE
```

## 5. Future utility and compression levels

Durability alone is insufficient. A candidate also receives a bounded future-utility assessment. Useful conceptual dimensions include:

```yaml
future_relevance: 0.00..1.00
stability: 0.00..1.00
uniqueness: 0.00..1.00
specificity: 0.00..1.00
recurrence_probability: 0.00..1.00
```

The LLM may propose values; deterministic policy validates and bounds them.

AIP should use the smallest representation that preserves useful meaning:

- **Level 0 — discard:** no durable storage.
- **Level 1 — historical trace:** about one short sentence.
- **Level 2 — compact fact:** one or a few structured fields.
- **Level 3 — important memory:** richer context around a durable fact or decision.
- **Level 4 — important event:** compact chronology and outcome; uncommon by design.

A low-priority historical fact can survive in one line without competing with current high-value context.

## 6. Atomic memories, summaries, tree, and graph

AIP should prefer atomic authoritative memories so individual facts can change, expire, be superseded, be retrieved, and carry confidence independently.

Example:

```text
user.hardware.cpu.current
user.hardware.gpu.current
user.hardware.gpu.history
user.projects.project_x.decisions
agent.identity
agent.relationships.owner
conversation.current.working_state
```

Memories must not live as one flat list. The primary organization is a semantic hierarchy, while relationship edges provide graph retrieval.

Example graph relations:

```text
current_device
  -> replaces -> previous_device
  -> belongs_to -> main_system
  -> related_event -> maintenance_event
  -> related_task -> tuning_task
```

Composite summaries may be derived from atomic memories for efficient context assembly, but summaries are never factual authority. When source memories change, affected projections and summaries must be rebuilt or invalidated.

## 7. Supersession, contradiction, confidence, and provenance

A mutable fact must support supersession.

Incorrect:

```text
current_device = A
current_device = B
```

Correct:

```text
A: historical / superseded
B: current
A -> superseded_by -> B
```

If AIP cannot determine which claim is correct, it must preserve an explicit unresolved conflict instead of overwriting one side.

Explicit Owner corrections override inference. Inferred knowledge must never silently become equivalent to an explicit Owner statement.

Every important durable memory should retain provenance such as:

```text
explicit_owner_statement
owner_correction
conversation_inference
agent_observation
tool_result
imported_profile
system_state
```

Confidence is independent from importance. A low-confidence inference can be important to inspect while remaining non-authoritative.

## 8. Temporary context is first-class

Temporary working state is not a poor-quality long-term memory. It is its own object.

```yaml
temporary_context:
  subject: example_tuning_task
  current_test:
    parameter: value
    status: testing
  expires_when:
    - test_completed
    - topic_abandoned
```

When work finishes:

```text
working state
  -> extract final useful result
  -> persist final result only if warranted
  -> delete / expire temporary details
```

Temporal semantics matter. The same numeric value can be temporary when the Owner is testing it and durable when the Owner states it has been the stable daily configuration for months.

## 9. Conversation history and recursive compaction

Conversation history and memory are separate systems.

Raw messages may remain available for user review, search, provenance, audit, and detailed retrieval. Old messages must not automatically enter each model call.

Long conversations should produce bounded summaries:

```text
turns
  -> chunk summary
  -> conversation summary
  -> topic / period summary
  -> profile projection
```

Repeated summarization must not become the only source of truth. Summaries retain provenance pointers so AIP can retrieve original messages when more detail is required.

The visible conversation can grow indefinitely while active context remains bounded:

```text
visible conversation length != active model context length
```

Topic segmentation should keep unrelated historical subjects out of the active context unless the current request references them.

## 10. Entity index and retrieval

AIP should maintain an entity index linking entities to memories, events, conversations, aliases, and current state.

Before significant generation, retrieval uses:

```text
new user message
+ current conversation topic
+ current state
+ relevant entity matches
```

A conceptual retrieval score may combine:

```text
semantic relevance
+ recency
+ importance
+ current-state relevance
+ entity match
+ conversation continuity
+ pinned-memory bonus
- contradiction penalty
- stale penalty
```

Retrieval must be bounded and task-specific. A hardware question should not automatically load unrelated project, game, network, or historical shopping context. A historical question should deliberately increase retrieval of historical branches.

## 11. Context Compiler

Before every model request, AIP compiles disposable runtime context.

```text
USER MESSAGE
   -> Intent / Entity Analysis
   -> Memory Retrieval
   -> Temporary-State Retrieval
   -> Conversation Retrieval
   -> Ranking
   -> Token-Budget Allocation
   -> Context Assembly
   -> LLM
```

Suggested default priority classes:

```text
P0 system / security constraints
P1 protected agent identity
P2 current user request
P3 current conversation turns
P4 current relevant state
P5 confirmed high-relevance memories
P6 relevant project / task context
P7 relevant relationship / goal context
P8 relevant historical memory
P9 relevant opinions
```

Lower-priority information is removed first under pressure.

The fully assembled prompt is ephemeral. By default AIP persists only necessary metadata such as memory IDs used, token counts, model used, pressure level, and retrieval diagnostics. It must not persist the full compiled prompt or hidden reasoning by default.

## 12. Dynamic token budget and pressure

AIP must not hardcode one universal context size. Budget derives from the selected model profile.

Before assembling input:

```text
max_context
- reserved_output
- safety_margin
= maximum_input_context
```

Output capacity is reserved first. Reasoning-heavy model profiles may reserve more generation capacity.

The runtime should expose internal context-pressure states such as:

```text
GREEN  < 60%
YELLOW 60-80%
ORANGE 80-90%
RED    > 90%
```

These are operational thresholds, not necessarily permanent UI labels. Pressure controls summarization, compaction, selectivity, and dropping low-priority context.

Context overflow must be prevented before the request reaches the model host. Waiting for an oversized-context error is not normal behavior.

## 13. Memory lifecycle, garbage collection, and pinning

Conceptual lifecycle:

```text
Detected
 -> Candidate
 -> Classified
 -> Compacted
 -> Validated
 -> Confirmed / Accepted
 -> Active
 -> Updated / Superseded / Archived / Expired
```

Candidate filtering should ask at minimum:

1. Is this actually information?
2. Who or what owns it?
3. Is it likely to matter again?
4. Is it temporary?
5. Is it already known?
6. Does it update an existing fact?
7. Can it be compressed further?
8. Is it sensitive?
9. Is it inferred or explicit?
10. Which namespace owns it?

Garbage collection periodically evaluates duplicates, obsolete temporary state, expired context, redundant summaries, superseded projections, low-value candidates, and orphaned derived data. Outcomes may be keep, compress, merge, archive, expire, or delete.

Pinned Owner memories cannot be automatically deleted. They receive elevated retrieval priority but may still have a compact representation.

Importance may evolve. Repeatedly useful details can be promoted; old diagnostic minutiae can be demoted while preserving the durable outcome.

## 14. Privacy and reasoning boundary

AIP never persists hidden chain-of-thought, internal token streams, or full private reasoning as long-term memory.

Persist only appropriate artifacts such as:

```text
decision
result
relevant evidence
source
confidence
```

Temporary-chat content must not silently cross into durable memory, profile projections, persistent summaries, or state-changing external actions.

## 15. Target storage model

Conceptual entities include:

```text
users
agents

memory_items
memory_edges
memory_events
memory_conflicts

user_profile_nodes
agent_profile_nodes

conversation_messages
conversation_chunks
conversation_summaries

temporary_context
working_state

entities
entity_aliases
entity_memory_links

context_assembly_logs
memory_processing_checkpoints
```

The exact schema is implementation-dependent, but the boundaries are not.

A target memory item should support at least:

```yaml
memory_item:
  id: opaque_id
  owner_type: user_or_agent
  owner_id: opaque_owner_id
  namespace: hardware.example.current
  type: fact
  content: structured_payload
  status: active
  durability: durable
  importance: 0.0..1.0
  future_utility: 0.0..1.0
  confidence: 0.0..1.0
  source:
    type: explicit_owner_statement
    conversation_id: optional
    message_id: optional
  supersedes: optional_memory_id
  created_at: timestamp
  updated_at: timestamp
```

## 16. Diagnostics and failure behavior

Development diagnostics should make context and memory behavior explainable.

Context diagnostics should expose, without leaking private content:

- selected model and context window;
- input budget, output reserve, and safety reserve;
- actual estimated input usage;
- token usage by source class;
- pressure state;
- IDs/classes of context sources used.

Memory diagnostics should answer:

- why a memory was created;
- why it was kept, compressed, merged, or omitted;
- why it was retrieved;
- what superseded it;
- which source created it;
- which policy version decided its lifecycle.

Graceful degradation rules:

- retrieval failure -> continue with reduced context;
- compaction failure -> use a deterministic fallback;
- unavailable summary -> retrieve bounded recent messages;
- token-estimation failure -> use a conservative budget.

## 17. Fundamental invariants

The implementation must preserve all of the following:

1. The complete conversation is never automatically injected into every prompt.
2. Long-term memory is semantic and compact, not transcript-shaped.
3. Temporary state does not automatically become durable memory.
4. Obsolete facts are superseded rather than remaining equally current.
5. Historical low-value information may survive in aggressively compressed form.
6. Current important information receives richer structured representation.
7. Context is assembled per request according to relevance.
8. Output tokens are reserved before input context is assembled.
9. Memory and active context remain bounded.
10. User Profile, Agent Profile, Memory, History, and Runtime Context remain separate concepts.
11. Raw hidden reasoning is never durable memory.
12. The LLM proposes interpretation; deterministic application logic owns persistence.
13. Important durable memories retain provenance.
14. Summaries are context aids, not factual authority.
15. A conversation may continue indefinitely without requiring its complete history to fit inside the model window.
16. User and agent memory namespaces remain isolated unless an explicit, authorized mechanism links them.
17. Profile projections and derived summaries are invalidated or rebuilt when authoritative memories change.
18. Context-window exhaustion is a prevented runtime condition, not a normal conversation ending.

## 18. Final architecture

```text
USER INPUT
   |
   v
Intent / Entity Pass
   |
   +-------------------+-------------------+
   |                   |                   |
   v                   v                   v
Recent Conversation  Working State     Memory Retrieval
   |                   |                   |
   +-------------------+-------------------+
                       |
                       v
                Context Compiler
             ranking + token budget
                       |
                       v
                      LLM
                       |
                       v
                   RESPONSE
                       |
                       v
            Memory Candidate Extraction
                       |
                       v
            Durability / Utility / Class
                       |
                       v
                  Compaction
                       |
                       v
       Duplicate / Update / Conflict /
                  Supersession
                       |
                       v
                Rust Policy Gate
                       |
                       v
              SQLite Knowledge DB
                       |
            +----------+----------+
            v          v          v
       User Profile Agent Profile Memory Graph
```

The persistent knowledge system is the continuity layer. The model context is only a request-scoped compilation of the minimum useful subset.