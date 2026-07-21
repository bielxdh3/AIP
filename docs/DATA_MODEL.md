# AIP Data Model

## 1. Principles

- SQLite is the authoritative local database.
- Rust owns migrations and persistence.
- IDs are opaque strings. UUIDv7 is the preferred initial format, subject to implementation support.
- Timestamps are stored in UTC with millisecond precision.
- Agent identity is independent from model identifiers.
- Temporary chat content never enters the database.
- Large binary assets are stored in the configured data root and referenced by validated relative paths.
- Schema changes use explicit numbered migrations.
- Foreign keys are enabled.
- User content is not copied into diagnostic logs.

## 2. Ownership model

v0.1 has one implicit Owner profile without authentication.

The schema must still support later local accounts and ownership transfer.

### `users`

| Field | Type | Notes |
|---|---|---|
| `id` | text PK | Opaque user ID |
| `role` | text | `owner`, later `member` |
| `display_name` | text | User-facing name |
| `created_at` | integer | UTC milliseconds |
| `updated_at` | integer | UTC milliseconds |
| `disabled_at` | integer nullable | Future account disable |

v0.1 inserts one Owner row during initialization.

## 3. Agents

### `agents`

| Field | Type | Notes |
|---|---|---|
| `id` | text PK | Stable identity |
| `owner_user_id` | text FK | Current owner |
| `name` | text | User-facing name |
| `species` | text | Free-form validated value |
| `pronouns_json` | text | Structured pronouns |
| `birthday` | text | Chosen calendar date |
| `initial_age_value` | real | Owner-selected initial age |
| `age_model` | text | Human, custom, timeless, etc. |
| `created_at` | integer | Real creation time |
| `origin_agent_id` | text nullable | Derived-copy lineage |
| `status` | text | `active`, `suspended`, `archived` |
| `main_conversation_id` | text nullable | Assigned after chat creation |
| `default_model_ref` | text nullable | Replaceable model reference |
| `safe_mode_only` | integer | Recovery flag when needed |
| `created_by_user_id` | text FK | Audit ownership |
| `updated_at` | integer | UTC milliseconds |

### `agent_traits`

| Field | Type | Notes |
|---|---|---|
| `agent_id` | text FK | Composite key |
| `trait_key` | text | Stable English identifier |
| `display_label_key` | text | Portuguese localization key |
| `value` | real | Normalized 0.0 to 1.0 |
| `is_protected` | integer | Later evolution cannot replace directly |
| `source` | text | Creation, owner edit, later evolution |
| `updated_at` | integer | UTC milliseconds |

Initial trait keys may include:

- `curiosity`
- `sociability`
- `criticality`
- `spontaneity`
- `affection`
- `autonomy`

### `agent_state`

One current row per agent.

| Field | Type | Notes |
|---|---|---|
| `agent_id` | text PK/FK | Agent |
| `sleep` | real | 0.0 to 1.0 |
| `energy` | real | 0.0 to 1.0 |
| `mood` | real | Normalized signed or mapped value |
| `mode` | text | `normal`, `voice_muted`, `silent`, `safe` |
| `is_awake_override` | integer | Temporary wake-now override |
| `last_simulated_at` | integer | Offline progression anchor |
| `updated_at` | integer | UTC milliseconds |

### `agent_screen_preferences`

| Field | Type | Notes |
|---|---|---|
| `agent_id` | text PK/FK | Agent |
| `monitor_id` | text nullable | Stable best-effort monitor reference |
| `preferred_x` | real | Logical coordinate |
| `preferred_y` | real | Logical coordinate |
| `always_on_top` | integer | Per-agent override |
| `desktop_only` | integer | Per-agent override |
| `hide_fullscreen` | integer | Per-agent override |
| `gravity_mode` | text | Configurable behavior |
| `surface_policy_json` | text | Taskbar/window/icon/agent settings |
| `updated_at` | integer | UTC milliseconds |

## 4. Conversations and messages

### `conversations`

| Field | Type | Notes |
|---|---|---|
| `id` | text PK | Conversation ID |
| `agent_id` | text FK | Owning agent |
| `owner_user_id` | text FK | Access boundary |
| `title` | text | User-facing title |
| `kind` | text | `normal`, `private` |
| `is_main` | integer | One main conversation per agent |
| `model_override_ref` | text nullable | Conversation override |
| `created_at` | integer | UTC milliseconds |
| `updated_at` | integer | UTC milliseconds |
| `archived_at` | integer nullable | Archive time |

Temporary conversations are not stored here.

### `messages`

| Field | Type | Notes |
|---|---|---|
| `id` | text PK | Message ID |
| `conversation_id` | text FK | Conversation |
| `agent_id` | text FK | Explicit isolation aid |
| `author_type` | text | `user`, `agent`, `system` |
| `author_user_id` | text nullable | User author |
| `content_text` | text | Message content |
| `content_format` | text | Initial value `plain_text` |
| `model_ref` | text nullable | Actual generation model |
| `generation_status` | text | `pending`, `streaming`, `complete`, `failed`, `cancelled` |
| `reply_to_message_id` | text nullable | Future threading |
| `created_at` | integer | UTC milliseconds |
| `completed_at` | integer nullable | Generation completion |

### `conversation_summaries`

| Field | Type | Notes |
|---|---|---|
| `id` | text PK | Summary ID |
| `conversation_id` | text FK | Conversation |
| `through_message_id` | text FK | Covered boundary |
| `summary_text` | text | Model-generated, not fact authority |
| `model_ref` | text nullable | Model used |
| `created_at` | integer | UTC milliseconds |

## 5. Memory

### `memories`

| Field | Type | Notes |
|---|---|---|
| `id` | text PK | Memory ID |
| `agent_id` | text FK | Owning agent |
| `owner_user_id` | text FK | Privacy boundary |
| `category` | text | Permanent, preference, fact, rule, emotional |
| `content_text` | text | Canonical memory statement |
| `status` | text | `active`, `archived`, `trashed` |
| `confirmation_status` | text | `confirmed`, `candidate`, `provisional`, `disputed` |
| `confidence` | real | 0.0 to 1.0 |
| `importance` | real | 0.0 to 1.0 |
| `source_type` | text | Manual, owner statement, extraction, internet, agent conversation |
| `source_message_id` | text nullable | Evidence pointer |
| `source_conversation_id` | text nullable | Evidence pointer |
| `supersedes_memory_id` | text nullable | Optional relation, never silent deletion |
| `created_at` | integer | UTC milliseconds |
| `updated_at` | integer | UTC milliseconds |
| `archived_at` | integer nullable | Archive time |
| `trashed_at` | integer nullable | Trash time |

### `memory_conflicts`

| Field | Type | Notes |
|---|---|---|
| `id` | text PK | Conflict record |
| `agent_id` | text FK | Agent |
| `memory_a_id` | text FK | First memory |
| `memory_b_id` | text FK | Conflicting memory |
| `resolution_status` | text | Open, confirmed A, confirmed B, contextual, unresolved |
| `resolution_note` | text nullable | Owner-visible explanation |
| `created_at` | integer | UTC milliseconds |
| `resolved_at` | integer nullable | UTC milliseconds |

## 6. Models

### `model_providers`

| Field | Type | Notes |
|---|---|---|
| `id` | text PK | Example `ollama` |
| `display_name` | text | Provider name |
| `endpoint` | text nullable | Advanced configuration |
| `enabled` | integer | Availability setting |
| `config_json` | text | Non-secret provider settings only |
| `created_at` | integer | UTC milliseconds |
| `updated_at` | integer | UTC milliseconds |

Secrets, when later required, must not be stored in plain provider JSON.

### `models`

| Field | Type | Notes |
|---|---|---|
| `ref` | text PK | Provider-qualified reference |
| `provider_id` | text FK | Provider |
| `provider_model_id` | text | Provider identifier |
| `display_name` | text | Friendly name |
| `capabilities_json` | text | Text, tools, vision, context, etc. |
| `plain_description_key` | text nullable | Portuguese UI copy key |
| `installed_state` | text | Installed, remote, unavailable, unknown |
| `last_seen_at` | integer nullable | Discovery time |
| `metadata_json` | text | Non-authoritative provider metadata |

### `model_benchmarks`

| Field | Type | Notes |
|---|---|---|
| `id` | text PK | Benchmark ID |
| `model_ref` | text FK | Model |
| `hardware_json` | text | Sanitized local summary |
| `context_size` | integer | Test context |
| `load_ms` | integer nullable | Load time |
| `first_token_ms` | integer nullable | TTFT |
| `tokens_per_second` | real nullable | Throughput |
| `observed_ram_bytes` | integer nullable | Observation |
| `observed_vram_bytes` | integer nullable | Observation |
| `quality_rating` | real nullable | Owner rating |
| `notes` | text nullable | Clearly local observation |
| `created_at` | integer | Test time |

## 7. Pixel art and appearance

### `pixel_documents`

| Field | Type | Notes |
|---|---|---|
| `id` | text PK | Source document |
| `owner_user_id` | text FK | Owner |
| `name` | text | Asset name |
| `category` | text | Body, hair, clothing, accessory, etc. |
| `schema_version` | integer | Format version |
| `source_relative_path` | text | Validated path under data root |
| `preview_relative_path` | text nullable | Derived PNG |
| `width` | integer | Initial 64 |
| `height` | integer | Initial 64 |
| `created_at` | integer | UTC milliseconds |
| `updated_at` | integer | UTC milliseconds |

### `agent_appearance`

| Field | Type | Notes |
|---|---|---|
| `agent_id` | text PK/FK | Agent |
| `appearance_json` | text | Selected layered assets and colors |
| `updated_at` | integer | UTC milliseconds |

The source JSON format is versioned and stored as a file. Database JSON references selected assets but does not replace source documents.

## 8. Runtime and queue metadata

### `runtime_settings`

Singleton or keyed settings include:

- Python executable or bundled runtime reference;
- Ollama endpoint;
- model keep-alive, default 15 minutes;
- basic resource limits;
- safe-mode startup preference;
- data root paths.

No secret values are stored unprotected.

### `generation_jobs`

Persist only if restart recovery is later required. v0.1 may keep active jobs in memory and store only completed message state.

Suggested fields:

- request ID;
- agent ID;
- conversation ID;
- model reference;
- priority;
- status;
- created time;
- started time;
- completed time;
- sanitized error code.

Do not persist raw assembled prompts unless a later explicit debugging mode is designed with privacy controls.

## 9. Audit

Audit is minimal in v0.1 and expands with tools.

### `audit_events`

| Field | Type | Notes |
|---|---|---|
| `id` | text PK | Event ID |
| `event_type` | text | Stable event key |
| `actor_user_id` | text nullable | User |
| `agent_id` | text nullable | Agent |
| `target_type` | text nullable | Resource class |
| `target_id` | text nullable | Resource ID |
| `result` | text | Success, denied, failed |
| `metadata_json` | text | Sanitized, no conversation content or secrets |
| `created_at` | integer | UTC milliseconds |
| `expires_at` | integer nullable | Later 30-day retention |

Temporary-chat audit must not contain conversation content.

## 10. Settings

### `settings`

| Field | Type | Notes |
|---|---|---|
| `scope_type` | text | Application, user, agent |
| `scope_id` | text | Scope identifier |
| `key` | text | Stable English key |
| `value_json` | text | Validated value |
| `updated_at` | integer | UTC milliseconds |

Use typed accessors. Do not scatter unvalidated arbitrary settings across the codebase.

## 11. Constraints and indexes

Required constraints include:

- one main conversation per agent;
- one current agent-state row per agent;
- conversation owner and agent must match message owner boundaries;
- memory owner and agent must match ownership boundaries;
- model references may become unavailable without deleting agent configuration;
- archived agents retain history;
- suspension does not rewrite birthday or age lineage;
- temporary chat must have no persistence path.

Recommended indexes:

- messages by conversation and created time;
- conversations by agent and updated time;
- memories by agent, status, category, and importance;
- model benchmarks by model and created time;
- audit by created and expiry time;
- agents by owner and status.

## 12. Migration rules

- Never auto-delete user data because a migration fails.
- Back up the database before high-risk migrations when backup support exists.
- Migration state is visible in diagnostics.
- A failed migration enters recovery or safe mode.
- Schema reset is a separate explicit user action.
- Model removal never cascades into agent or conversation deletion.
- User deletion behavior is not implemented until the multi-user phase is explicitly designed.

## 13. Phase 0 implemented subset

Migration `0001_phase0.sql` implements only `schema_migrations`, the implicit Owner row,
two provisional `agents`, `agent_screen_preferences`, and bounded `app_settings`. The Rust
data layer seeds stable provisional identifiers, persists safe mode and positions, enables
foreign keys on every connection, and applies the migration idempotently.

Conversation, message, memory, model, audit, export, and multi-user tables remain design-only.
