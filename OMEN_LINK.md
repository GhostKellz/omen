put the “smart cycling” brain in OMEN, not in Zeke, Glyph, or Rune.

Where the logic should live

OMEN (service/router) → Owns model selection, budget/quotas, latency/cost scoring, and “try local Ollama → escalate to cloud if needed.” It already speaks OpenAI-compat and is built for routing + auth + usage.

Zeke / zeke.nvim (client) → Sends hints (intent, size, latency target), persists local prefs & stats in a tiny zqlite (project/user cache), and displays costs. No provider keys or heavy policy here.

Glyph (MCP) → Tools (read/write/scan/search/apply_patch) + consent/audit; no routing.

Rune (Zig lib) → Fast file ops/diff/search; no routing.

GhostLLM → If you prefer, it can be the router instead of OMEN; don’t split responsibilities—pick one router.

Why OMEN?

Centralized view of budgets/quotas and historical latency across providers.

Can maintain per-project routing profiles and do sticky sessions.

Lets Zeke stay thin and fast; your 4090 (Ollama) is just another provider OMEN scores.

Minimal contract (Zeke ↔ OMEN)
Zeke sends (per request)
{
  "model": "auto",                        // or alias (code-fast, reason-deep)
  "messages": [...],
  "tags": {
    "intent": "code|tests|reason|vision",
    "project": "grim",
    "latency": "low|normal|high",
    "budget": "frugal|normal|generous",
    "size_hint": "tiny|small|medium|large"
  }
}

OMEN decides

Route to ollama:deepseek-coder:14b/33b for intent=code + size_hint∈{tiny,small}

Escalate to Claude Opus / Sonnet or Azure GPT-5 Codex for reason or large

Enforce quotas + session stickiness; return provider, model, latency_ms, cost_est

Zeke stores (local zqlite)

Per-project: last chosen alias → actual model, success/fail, TTFB, tokens_used

Simple prefs overrides (e.g., “prefer local for grim unless size_hint=large”)

Tiny local DB (zqlite) schema (client-side)
CREATE TABLE IF NOT EXISTS routing_prefs (
  project TEXT PRIMARY KEY,
  prefer_local INTEGER NOT NULL DEFAULT 1,
  max_cloud_cost_cents INTEGER DEFAULT 200,         -- per op soft cap
  last_alias TEXT, last_model TEXT, updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS routing_stats (
  id INTEGER PRIMARY KEY,
  project TEXT, alias TEXT, model TEXT,
  intent TEXT, size_hint TEXT,
  provider TEXT, latency_ms INTEGER, tokens_in INTEGER, tokens_out INTEGER,
  success INTEGER, created_at INTEGER
);


Use it only for hints and UX; the source of truth for budgets remains OMEN.

Practical rules (good defaults)

Default alias map

code-fast → ollama:deepseek-coder:14b (TTFB low, great for edits)

code-plus → ollama:deepseek-coder:33b

code-smart → azure:gpt-5-codex (or OpenAI 4o-mini)

reason-deep→ anthropic:claude-3.5/opus

Escalation

If local TTFB>2s or tokens_out>2k or tool_calls>1 → escalate to cloud.

If cloud 5xx/timeout → fall back to local.

Stickiness

Keep a session on the chosen provider until a rule is violated (saves tokens/latency).

Budgets

Zeke includes budget tag; OMEN enforces monthly/weekly caps and per-provider soft limits.

What to build with your remaining 22%

Today/Tomorrow (Zeke + zeke.nvim)

Implement model aliases + tags in Zeke requests.

Add local zqlite cache (schema above) and zeke doctor for Ollama/OMEN health.

zeke.nvim:

Model switcher UI + status badge.

Diff-apply via Glyph tools.

Visual-selection → context → request (with intent tags).

OMEN quick wins

Turn on router heuristics: prefer local for intent=code, escalate on size_hint=large.

Enable per-provider soft limits and session stickiness.

Expose X-OMEN-Trace headers (chosen provider/model, routing reason) so Zeke can log them.

Where each project lands (one-liner roles)

OMEN: Router, auth, quotas, intelligent cycling between Ollama and cloud.

Zeke/zeke.nvim: Client UX, context packing, local prefs cache, tags.

Glyph: Tooling (fs/shell/http) under consent/audit.

Rune: Fast file ops/diff/search via FFI inside Glyph/Zeke.

GhostLLM: Alternative to OMEN (pick one to avoid split-brain).
