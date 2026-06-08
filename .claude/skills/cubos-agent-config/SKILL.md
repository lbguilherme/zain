---
name: cubos-config
description: "Read, edit, and sync the declarative Cubos Core tenant config (TOML + Markdown) with the cubos-config tool / make. Use when working with files under config/tenants/ (agents, providers, MCPs, skills, triggers, channels, roles, knowledge bases), their fields, the agent IDENTITY/SOUL/CAPABILITIES/AGENTS/BOOTSTRAP docs, the env-var/@path/secret conventions, or the pull/push/deploy/status/diff/sync commands."
---

# Declarative tenant configuration (cubos-config)

A complete guide to configuring a Cubos Core tenant **as files** (TOML + Markdown) and syncing them with the server through `cubos-config` (the `config_sync` crate). It covers the on-disk layout, the conventions (slug, `${ENV}`, `@path`, opaque secrets, commented defaults), the `pull`/`push`/`status`/`diff`/`sync`/`deploy` commands, and **every entity and field** — providers, roles, MCPs, skills, knowledge bases, agents (including the five Markdown "soul" documents), triggers and channels. Most field descriptions below are the exact copy the dashboard shows, so the file and the UI stay in agreement.

`cubos-config` reads the files, talks to the server's **REST API**, and reconciles (create/update/delete) idempotently. `pull` (cloud → disk) and `push` (disk → cloud) round-trip cleanly: right after a `pull`, a `status` reports "everything in sync".

---

## 1. On-disk layout

Everything lives under `<config-dir>/tenants/<tenant-slug>/`. Each resource kind is a subfolder; each file is one resource.

```
config/
└── tenants/
    └── <tenant-slug>/            # e.g. zain — the FOLDER NAME is the tenant slug
        ├── tenant.toml           # the tenant's display_name
        ├── providers/<slug>.toml # LLM providers (gemini, ollama, tavily, …)
        ├── roles/<slug>.toml     # user roles (capability bundles per persona)
        ├── mcps/<slug>.toml      # MCP servers (tools)
        ├── skills/<slug>.md      # skills (markdown knowledge bundles)
        ├── knowledge-bases/<external_id>.toml
        ├── agents/<slug>.toml    # agents (the central resource)
        ├── docs/*.md             # markdown referenced by agents via @path
        ├── triggers/<slug>.toml  # triggers (schedulable automations / agent tools)
        └── channels/<slug>.toml  # channels (telegram / whapi / external)
```

Add a resource = create the file. Remove one = delete the file (and `push --delete`, see §3).

---

## 2. Conventions (apply to every entity)

- **The slug comes from the file/folder name — there is no `slug` field in the TOML.** `providers/gemini.toml` → slug `gemini`; the folder `tenants/zain/` → tenant `zain`; `skills/onboarding.md` → skill `onboarding`. Renaming the file renames the resource (it creates a new one and orphans the old).
- **`${VAR}` interpolation:** any string in a TOML containing `${NAME}` is replaced by the `NAME` environment variable at load time. A missing var fails fast. Used for every secret and sensitive URL (`api_key = "${GEMINI_API_KEY}"`).
- **`@path` markdown references (agents only):** the agent's five `*_md` fields accept `@relative/path.md` (relative to the `agents/` folder). The file's contents are sent inline, e.g. `identity_md = "@../docs/identity.md"`.
- **Opaque secrets:** the server **never returns** secrets on `pull` — `provider.api_key`, `mcp.headers`, `channel.bot_token`/`token`. On pull they are omitted (or rewritten back as `${ENV}` when the local file already had a matching placeholder). On push you **must** supply them (via `${ENV}` or a concrete value). See §5.
- **Commented defaults:** on `pull`, a field whose value equals the server default is written **commented out** (`# max_turns = 30`) — documenting the default without setting it. Only non-default values stay active.
- **Empty `""` / empty arrays** are treated as "absent" and never produce a diff.

---

## 3. Commands

Run via the `Makefile` (in the config repo) or directly: `cubos-config --config-dir config <cmd>`. Credentials come from `CUBOS_AGENT_URL` + `CUBOS_AGENT_ROOT_KEY` (root Bearer).

| Command | What it does |
|---|---|
| `make pull` | cloud → disk. Rewrites the tenant's TOML/MD from the server. **Discovers** tenants: without `--tenant`, it lists the cloud's tenants and materializes each. |
| `make push` | disk → cloud, **mirror**: creates/updates what exists locally **and deletes** on the cloud what isn't local (`--push-all --delete`). Tenants are never deleted. |
| `make status` | lists what's out of sync. Markers: `=` unchanged · `→` local-only (push creates) · `←` cloud-only (push skips, or deletes with `--delete`) · `≠` diverged. |
| `make diff` | field-by-field diff of drifted resources. |
| `make sync` | interactive, per resource: `[p]ush / [P]ull / [s]kip / [d]iff / [q]uit`; cloud-only also offers `[D]elete` (confirm by typing `delete`). |
| `make deploy` | **non-destructive** push (create-or-update, no deletes). Use for a safe push. |

**Pre-flight consistency check:** before any push (`--push-all` and `deploy`), the tool validates that every cross-reference resolves locally. If an agent references an mcp/skill/trigger/provider/role with no local file, it **fails before applying anything**, listing every problem:

```
✗ tenant 'zain' config is inconsistent — fix these references before pushing:
  - agent 'zain': binds trigger 'rwgwrt' (no triggers/rwgwrt.toml)
```

> Careful with `make push` (mirror): deleting a file deletes the resource on the server. If a file goes missing by accident, `--delete` removes it on the cloud. Use `make deploy` when you want a push that never deletes.

---

## 4. Entities and fields

### 4.1 `tenant.toml`

The slug is the folder name. Only field:

```toml
display_name = "Zain"
```

### 4.2 `providers/<slug>.toml` — LLM providers

Credentials + config for one external LLM provider. On create, the server **probes** the upstream (validates the key/URL) — an invalid key errors with 502.

```toml
provider_type = "gemini"          # gemini | ollama | ollama-cloud | lmstudio | openai | openai-compatible | tavily
base_url = "http://pc:11434"      # required for ollama, lmstudio, openai-compatible; rejected for the managed ones (gemini, ollama-cloud, openai, tavily)
api_key = "${GEMINI_API_KEY}"     # required for gemini, ollama-cloud, openai, openai-compatible, tavily; optional for lmstudio; rejected for ollama — OPAQUE ("set, type to replace")

# Per-model overrides (optional) — only the models you want to customize.
# "Override the auto-discovered price for any line item. Empty input falls back to the auto value."
[[model_overrides]]
model = "gemma4:e4b-it-q4_K_M"            # model id from the provider's discovered catalog
context_window_tokens_override = 16384    # effective context window — only meaningful for local providers (ollama / lmstudio)
# pricing_override = { input = 0.0000015, output = 0.000006 }   # USD per unit, per SKU (full precision)
```

The model catalog is **discovered** automatically (each model carries capability badges shown in the UI): `chat` (with `in`/`out`/`ctx` token limits), `tools`, `vision`, `thinking`, `embedding (<dims>, <ctx>)`, `websearch`, `transcription`. A capability the provider didn't report shows with a `?` ("unknown — provider didn't report this capability"). Models without overrides simply don't appear in `model_overrides`.

### 4.3 `roles/<slug>.toml` — user roles

A capability bundle per persona. Roles gate which MCPs / skills / triggers a user (and the agent acting for them) can use, and which roles are auto-assigned to new channel users. *"Each role carries its own MCPs / skills, so this is how you give newcomers a constrained capability bundle."*

```toml
display_name = "Manager"
description = "What this role unlocks"   # optional
```

### 4.4 `mcps/<slug>.toml` — MCP servers (tools)

A Model Context Protocol server that exposes tools to the agent. Creating one probes the URL (`tools/list`) first.

```toml
display_name = "Zain MCP"
transport = "http"                # default "http"
url = "http://pc:8088/"           # HTTP endpoint
dynamic_tools = true              # default false

# Optional HTTP headers, encrypted at rest — OPAQUE; "edits replace the whole bag". Values accept ${ENV}:
[headers]
"x-zig-api-key" = "${ZIG_API_KEY}"

# Tools disabled globally on this MCP (default: all enabled). Only the disabled ones are listed.
disabled_tools = ["dangerous_tool", "legacy_export"]
```

- **`dynamic_tools`:** *"Re-fetch the tool list from the server at the start of every turn instead of using the cached snapshot. Useful for servers that hide tools based on conversation context. Tools disabled here and the agent's allowlist still apply."*
- **`headers`:** *"Stored values are encrypted and cannot be displayed."* On pull they're omitted; a replacement swaps the whole header bag.
- **`disabled_tools`:** the operator's global on/off per tool (the agent's per-binding allowlist in §4.7 narrows further). Discovered tools can be flagged by the server with hints: `destructive`, `read-only`, `idempotent`, `open-world`.
- **Request metadata (context, not configurable):** every JSON-RPC request the agent sends carries a fixed `_meta.cubos-agent` envelope (`userId`, `userExternalId`, `userRoleSlugs`, `conversationId`, `tenantSlug`, `channelType`, `agentSlug`, `conversationMetadata`) so the MCP can identify the caller, the tenant and the user's roles. No opt-out, no per-server customization.

### 4.5 `skills/<slug>.md` — skills

*"A skill is a markdown bundle (SKILL.md + optional references). The description appears in the system prompt; the body is fetched on demand."* It complements MCPs (skill = knowledge, MCP = executable tools). The **whole file** is the skill:

- **slug** = file name without `.md`.
- **display_name** = the first `# heading`.
- **description** = the first paragraph after the heading. *"Appears in the `<skills>` block of the system prompt so the agent knows when this skill is relevant and worth loading."*
- **body** = the full markdown.

```markdown
# Churrascada Glossary

Internal operational terms the agent needs to understand.

## Locations
- "Matriz" = the Av. Paulista unit...
```

### 4.6 `knowledge-bases/<external_id>.toml` — knowledge bases

The file name is the **external_id** (public id, unique per tenant). *"A knowledge base groups facts about an entity — a project, an establishment, a customer account. Conversations and users attach to it; the agent reads and writes here alongside personal memory."*

```toml
display_name = "Acme Project"
# "The routing prompt is what the LLM router reads when deciding whether to write to this knowledge base."
routing_prompt = "Record facts about the Acme project here: deadlines, stakeholders, decisions."
```

### 4.7 `agents/<slug>.toml` — agents (the central resource)

The richest file. By section:

**Basics:**
```toml
display_name = "Zain"
# Description (for other agents): "One or two sentences describing what this agent is good at, written
# for other agents that may delegate to it. Shown to sibling agents when they decide whether to delegate
# work here. This agent itself never sees its own description."
# description = ""
provider_slug = "gemini"                 # chat-model provider (needs providers/gemini.toml)
model = "gemini-3.1-flash-lite"          # chat model name (resolved to a model_id on push)
# Max turns per response: "Upper bound on how many back-and-forth steps the agent can take to answer one
# user message. If it hits this cap, the reply is aborted with an error so a runaway loop can't burn the
# budget. Range: 1–200."
# max_turns = 30
# Execution timeout: "How long (in seconds) a single code-execution run is allowed to take before it's
# killed. Any HTTP fetches or tool calls the script does count against the same budget. Bounds: 1–600."
# code_execution_timeout_seconds = 30
# allowed_subagent_slugs = []            # see "Sub-agents" below
```

**Extra models (optional; each may live on a different provider than chat).** The `*_provider` defaults to `provider_slug`:
```toml
# Search model: "Picks where the agent runs searches when it needs to look something up on the open web.
# Leave unset to let the agent only read URLs it already knows; pick a search-capable model to discover new ones."
websearch_provider = "tavily"
websearch_model = "fast"
# Embedding model: "Picks the model the agent uses to look things up in memory by meaning, not just by
# keyword. Required while structured memory is on."
embedding_provider = "gemini"
embedding_model = "gemini-embedding-001"
embedding_dimensions = 768               # one of 384 / 768 / 1024 / 1536 / 3072
# STT model (audio transcription): "Lets the agent receive voice notes from chat channels by turning them
# into text first. Leave unset if this agent only deals with typed messages."
stt_provider = "gemini"
stt_model = "..."
```

**Features** — *"Toggle which built-in capabilities this agent gets. When a feature is off, the agent stops being able to do it — it won't even know the option existed."* All default `false`:

```toml
feature_task_planning = true
feature_web_search = true
feature_code_execution = true
feature_user_notes = true
feature_structured_memory = true
feature_proactive_memory = true
feature_subagent = true
```

- **`feature_task_planning`** — *"The agent maintains a visible checklist as it works — adding items, marking them in-progress, done, or cancelled — so multi-step requests don't lose their thread. Turn it on for agents that handle multi-stage requests (research, debugging); leave it off for purely conversational use where a plan would just be noise."*
- **`feature_web_search`** — *"The agent gains two abilities: run a search query and read back the top results (titles, snippets, links), and open any specific URL — pages the user mentions, links from search results, or public JSON APIs — and read the full content. Turn it on when answers depend on current or external information."* (Needs `websearch_model`.)
- **`feature_code_execution`** — *"The agent can write and run short programs during a turn — precise math, parse and reshape data, call APIs and process results inline, or chain several actions in one shot. Turn it on when the agent works with data, integrations, or precision/composition tasks."*
- **`feature_user_notes`** — *"The agent keeps a free-form notepad for each end user. It reads it at the start of every conversation and can write stable facts (name, preferences, ongoing context, decisions) so the next chat picks up where the last left off."*
- **`feature_structured_memory`** — *"The agent gets a queryable knowledge base it builds over time — one entry per fact/event/attribute, with context preserved. It records new entries and searches existing ones by topic; a contradicted entry is superseded, not deleted, so history stays auditable. Searches blend semantic similarity and keyword match."* (Needs `embedding_model`.)
- **`feature_proactive_memory`** — *"Before each turn, the system itself searches structured memory using the user's recent messages and feeds the most relevant entries into context — the agent doesn't have to decide to look. Costs one extra search per turn."* (Needs `feature_structured_memory`.)
- **`feature_subagent`** — *"The agent can hand off focused tasks — research, breaking a problem down, deep analysis — to a helper that works on the side and reports back. By default the helper is a fresh copy of itself; you can also let it call other agents from this tenant (via `allowed_subagent_slugs`), each with its own description."*

**The five Markdown documents — the agent's "soul".** Each field accepts `@path` (recommended) or inline text. They're assembled into the system prompt in this order:

```toml
identity_md     = "@../docs/identity.md"
soul_md         = "@../docs/soul.md"
capabilities_md = "@../docs/capabilities.md"
agents_md       = "@../docs/agents.md"
bootstrap_md    = "@../docs/bootstrap.md"
```

| Doc | What it is (dashboard copy) | Good practice |
|---|---|---|
| **IDENTITY.md** (`identity_md`) | *"Who the agent is. The first thing the model reads — sets the persona's name, role, employer, and the user it's talking to. Use it to answer, in the user's voice: 'if I asked, who am I talking to?'"* | Keep it short and concrete (2–5 lines). Avoid behavior rules (those go in AGENTS.md) and skill claims (those go in CAPABILITIES.md). If you switch which company/team the agent represents, this is the file you change. |
| **SOUL.md** (`soul_md`) | *"Personality, voice, tone — how the agent speaks, independent of what it knows. Cover register (formal vs casual), pacing (terse vs expansive), warmth (empathetic vs matter-of-fact), and humor."* | Express preferences as positive guidance plus the failure mode to avoid ("warm but never saccharine"). If two agents share skills but should *feel* different, this is the only file that differs. |
| **CAPABILITIES.md** (`capabilities_md`) | *"What the agent can do and how, in prose. List skills, typical workflows, and — equally important — the boundaries of competence. Descriptive context for the model, not the executable tool surface (MCPs handle that)."* | Mention the playbooks to reach for, the data sources it can cite, and situations it should hand off rather than attempt. Update it whenever you add/remove a tool so the model's self-model stays accurate. |
| **AGENTS.md** (`agents_md`) | *"Operating rules and house conventions. CLAUDE.md-style: hard constraints, things that must always or never happen, regardless of how the user asks."* | Imperative voice, prefer NEVER/ALWAYS. Group by topic (security, escalation, formatting). Treat it as a contract; write exceptions down explicitly so the model doesn't invent its own. |
| **BOOTSTRAP.md** (`bootstrap_md`) | *"Onboarding script for the start of a new conversation. A checklist the agent works through across as many turns as it takes — greeting, identification, scoping questions, consent gates — and decides on its own when onboarding is complete."* | List what the agent must know before moving on and the questions it must ask; let the model improvise wording/order. Leave empty if the channel arrives with enough context (webhooks, authenticated sessions). |

**Bindings (which resources the agent uses) — lists of slug:**
```toml
mcps = ["zain"]                          # enabled MCPs (must exist locally)
skills = ["zain-glossary"]               # enabled skills
triggers = ["daily-report"]              # triggers the agent may call as a tool
```

**Fine-grained config (optional) — per-tool and per-role control.** By default a bound MCP exposes all its tools to "All users":
```toml
# Per-MCP tool allowlist (omit an mcp = all its tools enabled):
[mcp_enabled_tools]
zain = ["get_places", "get_overview"]

# Per-role config on an agent-mcp (omit enabled_tools = all tools for that role):
[[mcp_roles]]
mcp = "zain"
role = "manager"
enabled_tools = ["get_places"]

# Which roles may use each skill / trigger (default: all users):
[skill_roles]
zain-glossary = ["manager"]
[trigger_roles]
daily-report = ["manager"]

# Auto-called tools: argument-less tools that run at the start of every response as live context:
[[auto_call]]
mcp = "zain"
tool = "get_client_state"
```

### 4.8 `triggers/<slug>.toml` — triggers (automations)

A parameterized task run by an agent, schedulable (cron / one-shot) and optionally callable as a tool.

```toml
display_name = "Daily Report"
# description = ""                       # "Shown to the agent as the tool's description." Only needed when exposed as an agent tool.
agent_slug = "zain"                      # "The agent that runs the background task on each fire." (must exist)
# "The task launched on each fire. Use {{param}} placeholders — detected parameters appear below."
prompt_template = """
Generate the daily report about {{topic}}. Send it to the user.
"""
default_cron_expr = "0 9 * * *"          # "Empty = one-shot. e.g. 0 9 * * * for daily at 09:00." (5 fields)
# timezone = "UTC"                       # "IANA timezone the cron is evaluated in." (default UTC)
# caller_can_schedule = true             # "Whether an activation may set its own fire time / cron." (default true)
min_period_minutes = 60                  # "Cadence floor; blocks schedules firing too often."
# default_stop_prompt = "..."            # "Natural-language condition the agent uses to stop a recurring run."
# default_max_runs = ...                 # "Hard cap on the number of fires for a recurring run."
# max_active_runs_per_user = ...         # "Blocks activating more once a user has this many active schedules of this trigger." (empty = unlimited)

# Declared parameters: "All required, substituted as text." They become {{...}} chips and tool args.
[[parameters]]
name = "topic"
description = "What the report is about"   # "what this parameter is for"
```

> The trigger's **API access secret** (`X-Trigger-Secret`) and the *Run now* activations are runtime/opt-in and **not** managed by config — generate/rotate them in the dashboard or via API. The scheduling fields above are *defaults* that pre-fill an activation's schedule; they don't fire the trigger by themselves.

### 4.9 `channels/<slug>.toml` — channels

An external service that talks to an agent. Three types, discriminated by `type`.

```toml
type = "telegram"                        # telegram | whapi | external
agent_slug = "zain"                      # the agent that answers this channel
user_onboarding_mode = "open"            # open | approval_required
# default_user_role_slugs = ["..."]      # roles auto-assigned to new users from this channel
# disabled = false                       # default false (soft-disable: "Stop worker")

# Per-type secret (OPAQUE; required on CREATE):
bot_token = "${TELEGRAM_BOT_TOKEN}"      # telegram only — "Replacing the token re-probes Telegram and updates the cached bot identity."
# token = "${WHAPI_TOKEN}"               # whapi only — "Replacing the token re-probes Whapi and re-registers the inbound webhook."
# (external has no inbound secret; the server issues a connection secret, shown once)
```

- **User onboarding** — *"Decides what happens when this channel sees a brand-new identifier."*
  - **`open`**: *"New users are created automatically and can talk to the agent right away. The default."*
  - **`approval_required`**: *"The first inbound message creates a pending user but is dropped. The user shows up under Users → Pending until you approve them; only then do their next messages reach the agent."*
- **`default_user_role_slugs`** — *"User roles automatically assigned to every brand-new user that arrives through this channel on first contact. Each role carries its own MCPs / skills, so this is how you give newcomers a constrained capability bundle by default."*
- **Secrets** — telegram needs `bot_token`, whapi needs `token`; external has none. Because the server never returns the secret, **creating a new channel requires supplying it** (the tool errors clearly if it's missing); updating an existing channel doesn't need to resend it. For external channels, the connection secret (and its rotation) is server-issued and shown once — not config.

---

## 5. Secrets

Opaque fields (never returned on pull): `provider.api_key`, `mcp.headers`, `channel.bot_token`, `channel.token`.

- **On push:** supply via `${ENV}` (recommended — `api_key = "${GEMINI_API_KEY}"` + export the var) or a concrete value (the `config/` dir is usually gitignored/local).
- **On pull:** if the local file already had `${ENV}` and the resolved value matches the cloud, the placeholder is preserved; otherwise the field is omitted. `--unsafe-secrets` forces writing the concrete cloud value (emergencies only).
- **On diff:** secrets don't count as drift by default (the cloud doesn't return them). `--include-secrets` includes them.

---

## 6. Common workflows

- **Start from the server:** `make pull` materializes the tenant under `config/`. Edit the files. `make status` / `make diff` show what changed. `make deploy` (safe) or `make push` (mirror) applies.
- **Add a resource:** create the file (`triggers/new.toml`), reference it where needed (e.g. `triggers = ["new"]` on the agent), `make push`.
- **Remove a resource:** delete the file **and** drop every reference to it (otherwise the consistency check blocks the push). `make push` (with `--delete`) removes it on the cloud — or `make sync` and pick `D`.
- **Duplicate a tenant:** copy the folder `tenants/<a>/` to `tenants/<b>/`, adjust `tenant.toml`, **re-supply the secrets** (pull omits them), `make push --tenant b`.
- **Sanity round-trip:** after any `pull`, `make status` should report "everything in sync".
