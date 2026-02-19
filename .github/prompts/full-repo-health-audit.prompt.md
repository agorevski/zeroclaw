/fleet Audit this repository's health using the @repo-health-auditor agent. Use AGENTS.md as the scoring rubric.

Run these 9 audits in parallel, one document per dimension under `audit/`:

1. `audit/01-trait-factory-boundaries.md` — Trait/Factory Boundary Integrity
2. `audit/02-binary-size-footprint.md` — Binary Size & Runtime Footprint
3. `audit/03-security-surfaces.md` — Security Surfaces
4. `audit/04-config-schema-stability.md` — Config Schema Stability
5. `audit/05-extension-point-health.md` — Extension Point Health
6. `audit/06-test-coverage-risk-tiers.md` — Test Coverage by Risk Tier
7. `audit/07-docs-multilingual-parity.md` — Docs System & Multilingual Parity
8. `audit/08-dependency-supply-chain.md` — Dependency Supply Chain
9. `audit/09-dx-ci-health.md` — DX & CI Health

Each document must follow the agent's output format: Health Summary → Key Findings (with Issue/Evidence/Risk Tier/Impact/Direction tables) → Strengths.

After all 9 are complete, produce `audit/00-executive-summary.md` that consolidates: overall health posture, top-10 risk register ranked by likelihood × impact, and prioritized quick wins vs structural improvements.
