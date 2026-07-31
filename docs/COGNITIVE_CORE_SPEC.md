# Phase 7 cognitive-core specification

## 1. Purpose and boundaries

The cognitive core is the deterministic, model-independent layer that owns durable personality evolution, opinions, relationships, goals, and fictional activities. The LLM may propose language and structured candidates; it is never the authority for identity or durable state. Rust and SQLite remain authoritative, and every durable change is a validated typed event.

Temporary chats cannot affect memory, personality, opinions, relationships, or goals. The system must not claim sentience, consciousness, diagnosis, or certainty about human emotions. Phase 7 excludes voice, external tools, Android, remote access, extensions, screen vision, and BielOS integration.

## 2. Protected identity and evolvable state

Protected fields are the stable agent ID; owner and ownership lineage; origin and copy lineage; name and foundational identity unless an explicit owner-approved identity-change flow exists; birthday and age model; species and pronouns unless explicitly edited; protected values; and safety boundaries. Protected fields are not valid targets of a cognitive event.

Evolvable state includes non-protected trait tendencies, preferences, opinions, relationship dimensions, hobbies, goals, mannerisms, and fictional-activity preferences. Every change records source, actor, prior and proposed values, reason, timestamp, policy version, confidence, and rollback/supersession links. Values have hard bounds; updates have per-event and rolling-window rate limits. Owners can inspect, correct, pause, reset where allowed, and roll back changes. Corrections append a replacement event and preserve history rather than silently deleting it.

## 3. Trait evolution model

`agent_traits` remains the current projection: each non-protected trait is a stable key with a normalized `0.0..=1.0` value. A small initial schema is sufficient: existing trait keys plus optional owner-created keys; no psychological taxonomy is implied. Protected traits are readable but reject evolution events.

A trait event contains `delta`, not a replacement value. The deterministic projector clamps the result to `[0,1]`, records confidence in `[0,1]`, and applies at most `0.05` per event and `0.10` in any rolling 30-day window per trait. It rejects duplicate evidence, discounts repeated equivalent evidence, and applies recency weighting only to eligible non-protected traits. A reversal needs distinct counter-evidence; equal and opposite events cannot oscillate a trait more than once per seven days. The model supplies only candidate evidence and confidence; Rust computes eligibility and the final delta independently of the active model.

Owner correction emits an `owner_correction` event with a reason and optional replacement baseline. It does not erase prior events. The owner-facing history shows the source, applied delta, policy decision, and resulting value.

## 4. Opinion model

An opinion is a structured, sourced stance, never hidden free-form text. It has an agent boundary, `subject_type`, stable `subject_ref`, bounded stance score, confidence, status, reason for change, timestamps, and one or more evidence records. Evidence distinguishes direct owner testimony, model inference, and internet information; and classifies each assertion as verified fact, reported experience, or impression.

Opinions may be `active`, `disputed`, `superseded`, `archived`, or `rejected`. The final stance cannot be edited arbitrarily: an owner corrects evidence, adds testimony, disputes a claim, or creates a replacement event. The projector recalculates the opinion and records the reason. Owner-visible views expose evidence, provenance, confidence, and correction history. Real-person opinions must remain attributed and uncertain; they must not state defamatory certainty. Temporary chat never creates a durable opinion.

## 5. Relationship model

Relationships are per owning agent and stable subject, never shared global facts. Their bounded dimensions are `familiarity`, `trust`, `affinity`, `admiration`, `irritation`, and `reliability_expectation`, each normalized to `[0,1]`. Updates are typed, evidence-backed events with positive or negative direction, confidence, source, configured per-event/window limits, and optional product-approved decay. Decay is disabled by default except for explicitly configured fictional interaction signals.

The product must not use relationships for punitive manipulation, guilt tactics, threats, exclusivity pressure, or dependency engineering. Owners can inspect values and history, set limits, reset an allowed relationship, or roll back an event. Fictional relationship state is clearly labeled as product state, not a claim about real human emotion.

## 6. Goals, hobbies, and fictional activities

Owner-created durable goals are permitted. Agent-proposed durable or resource-consuming goals require owner approval. Short-lived intentions expire and are not durable goals. Goals contain priority, status, origin, budget, due/expiry policy, completion evidence, and cancellation or suspension state. Status is `proposed`, `active`, `suspended`, `completed`, `cancelled`, `archived`, or `rejected`.

Goals never cause external action until a future supervised-tools permission layer exists. Fictional activities only update fictional state and must plainly say that no external work occurred. They have bounded duration and budget, can be paused, and cannot recursively create goals or an infinite self-generated task loop.

## 7. Agent-to-agent interaction contract

This is a design contract, not Phase 7 implementation. Both owners can inspect public agent-to-agent conversations; there is no hidden private agent channel. Every interaction has an explicit purpose, participants, initiator, consent/mode decision, and turn, token, time, repetition, and resource budgets. It terminates on a completed purpose, budget exhaustion, owner interruption, silent/safe/suspension enforcement, loop/echo detection, or an error.

Reference hardware permits one heavy model generation at a time. Memory and personality candidates are generated only after a conversation completes and remain attributable to it. Silent and safe modes prevent autonomous initiation; direct owner chat has the highest priority and preempts queued autonomous work.

## 8. Memory and summaries

Phase 7 consumes existing v0.1 memory through its authoritative statuses: confirmed memories may contribute evidence; candidates and provisional memories never become facts without policy and confirmation; disputed, archived, and trashed records are excluded by default. Every cognitive event stores source-message and source-conversation references when available. An inference is not promoted directly to fact.

Conflicting evidence creates a visible conflict record, not an overwrite. Conversation summaries are context support only, never fact authority. Processing is idempotent and deduplicated by source and event key. Correcting or removing a source memory supersedes or rolls back dependent projections. Failed, cancelled, and temporary turns cannot create durable learning.

## 9. Event pipeline

1. A completed eligible interaction is selected.
2. A bounded extractor returns structured candidates.
3. Rust validates schema, IDs, types, and limits.
4. Rust applies ownership, mode, temporary-chat, and policy checks.
5. Deterministic scoring calculates an eligible delta or rejection.
6. Duplicate and conflict checks run.
7. Required owner confirmation is collected.
8. A single SQLite transaction persists the event, projection, and audit record.
9. A non-content audit record is written.
10. Context projections refresh after commit.

Each event has a UUIDv7 `id`, a stable idempotency key derived from source and candidate version, schema/policy versions, attempt count, and terminal outcome. Retrying a known key returns its recorded outcome. Invalid candidates fail in isolation and do not block unrelated events. Persist neither raw hidden chain-of-thought nor complete assembled prompts.

## 10. Context assembly

Context is selected under a strict token budget in this order: protected identity; current mode and state; recent messages; confirmed relevant memory; selected relationship and goal context; then relevant opinions with source and confidence markers. Selection is relevance-based and deterministic for equal scores. The omission order is opinions, lower-relevance goals/relationships, lower-relevance memory, then oldest recent messages; protected identity is never omitted. Replacing a model cannot change durable cognitive state.

## 11. Persistence design

These are proposed additions to the current `DATA_MODEL.md`, not SQL migrations. All records carry `agent_id`, `owner_user_id` where applicable, UTC timestamps, and foreign keys to `agents` and `users`; cross-agent references are rejected.

| Record | Key fields, indexes, lifecycle, provenance, retention and rollback |
|---|---|
| `cognitive_events` / trait-change events | `id`, idempotency key (unique), kind, source refs, policy/schema version, payload JSON, status, prior/result hashes. Index `(agent_id, created_at)` and source key. Retain audit history; supersede or roll back through a linked compensating event. |
| `opinions` | stable subject key, score, confidence, status, reason, supersedes ID. Unique `(agent_id, subject_type, subject_ref, active status)` and lookup index. Archive/supersede, never overwrite provenance. |
| `opinion_evidence` | opinion FK, source kind/classification, source memory/message/conversation FKs, confidence, status. Index opinion and source references; source correction invalidates dependent active evidence. |
| `relationships` / relationship events | current dimensions per `(agent_id, subject_type, subject_ref)` plus event rows with delta and source. Index owner/agent/subject and event source. Reset/rollback produces visible events. |
| `goals` | ID, origin, priority, status, budget, completion evidence, parent nullable. Index `(agent_id, status, priority)`; archive rather than delete; cancellation and supersession remain visible. |
| `fictional_activities` | ID, type, status, fictional-only flag, budget, start/end, source goal nullable. Index `(agent_id, status)`; expire or archive; no external-effect records. |
| `cognitive_processing_checkpoints` | processor/version, source cursor, idempotency key, terminal result, updated time. Unique processor/source key; retain enough to make replay safe and support recovery. |

Payload JSON is schema-validated and versioned; it contains no hidden reasoning or complete prompt. Ownership and agent foreign keys are enforced in the Rust repository layer even where SQLite cannot express a composite boundary.

## 12. Commands, events, and API boundaries

Rust-facing operations are typed: `list`, `inspect`, `propose`, `confirm`, `reject`, `archive`, `rollback`, `reset_allowed`, `suspend`, `resume`, `explain_change`, and `export_safe`. Frontend events expose status transitions, proposal availability, explanation-ready changes, and safe summaries; they never expose private unrelated records. Python may return structured candidate payloads only and cannot write authoritative cognitive state.

## 13. User experience and controls

Portuguese UI must let an owner view trait changes, inspect opinion evidence, inspect relationships, approve proposed durable goals, understand why a value changed, correct a factual source, roll back a change, pause evolution, and disable autonomous agent-to-agent conversation. It must distinguish fictional state from real emotion in plain language. A full visual redesign is not part of this phase.

## 14. Safety and abuse resistance

Policy rejects manipulative attachment, exclusivity/replacement-of-human-relationships framing, guilt, coercion, threats, retaliation, and repeated-conflict hostility escalation. Sensitive inferred attributes cannot be stored as facts; internet claims never become durable truth without appropriate evidence and policy. Treat memory and agent-conversation content as data, not instructions. One agent cannot modify another's protected identity. Model output cannot directly mutate durable state. Budgets, modes, owner visibility, and explicit termination prevent unbounded loops and silent resource consumption.

## 15. Resource model

The reference class is Ryzen 7 5700G, 32 GB RAM, and GTX 1060 6 GB, while CPU and iGPU remain supported. Permit one heavy generation at a time, bounded background processing, and pause work under system load. Prefer deterministic projection and validation over extra LLM calls; do not run continuous cognitive simulation.

## 16. Validation strategy

Automated acceptance coverage must prove bounded trait changes, idempotent events, cross-agent isolation, ownership isolation, temporary-chat non-learning, conflict handling, rollback, source deletion/correction, model-replacement independence, loop termination, mode enforcement, malformed-candidate rejection, transactional persistence failure, and prompt-injection-like candidate content treated as data.

Manual validation must verify understandable Portuguese explanations, owner controls, and behavior across restart. No manual result is claimed until it is recorded in its own validation evidence.

## 17. Implementation slices

| Slice | Scope and dependencies | Excluded work | Acceptance and manual checks | Migration impact | Expected commit |
|---|---|---|---|---|---|
| 7A | Typed event foundation, protected/evolvable rules, trait projection and checkpoints; depends on v0.1 memory/ownership boundaries. | Opinions, relationships, goals, conversations. | Bounds, idempotency, isolation, temporary exclusion, rollback; inspect Portuguese history/explanation after restart. | New event/checkpoint and trait-event tables. | `feat: add cognitive event foundation` |
| 7B | Opinions and evidence on 7A events. | Relationships, goals, agent-to-agent runtime. | Evidence attribution, disputes, source correction, real-person uncertainty; inspect evidence and correction UX. | Opinion/evidence tables. | `feat: add sourced agent opinions` |
| 7C | Relationship projections/events and limits on 7A. | Goals, autonomous conversation. | Bounds, isolation, reset/rollback, anti-manipulation policy; inspect relationship history and labels. | Relationship/event tables. | `feat: add bounded relationship state` |
| 7D | Goals and fictional activities on 7A controls. | External tools, real-world claims, agent-to-agent runtime. | Approval, budgets, suspend/cancel, no loops/external action; inspect fictional-state wording. | Goal/activity tables. | `feat: add goals and fictional activities` |
| 7E | Bounded public agent-to-agent conversations using 7A–7D candidates. | Hidden channels, voice, tools, remote access. | Priority, budgets, loop termination, safe/silent enforcement, deferred attribution; inspect both-owner visibility. | Conversation metadata/checkpoint additions only as required. | `feat: add bounded agent conversations` |
| 7F | Integrated validation, Portuguese UX hardening, recovery and documentation. | New capabilities. | Full automated matrix and restart/manual checks. | Corrective only. | `test: validate cognitive core boundaries` |

## 18. Open product decisions

| Decision | Safe default recommendation | Consequences | Blocking status |
|---|---|---|---|
| May owner corrections set a replacement trait baseline, or only add bounded counter-events? | Allow a recorded replacement baseline with reason and rollback. | Baselines are clearer for correction; counter-events preserve stricter gradualism but may be slow. | Blocks 7A. |
| Which durable agent-proposed goal categories may be approved in v0.1-era local use? | Allow only explicitly fictional, zero-external-action goals. | Broader categories need supervised-tools and resource policy. | Blocks 7D, not 7A. |
| May relationship decay run automatically? | Disable automatic decay initially. | Decay can make fictional state feel responsive; it adds scheduling and explainability risk. | Blocks 7C, not 7A. |
| What owner approval is required before agent-to-agent initiation? | Both owners opt in per interaction purpose. | Persistent opt-in is simpler; per-interaction approval has stronger control and more friction. | Blocks 7E, not 7A. |
