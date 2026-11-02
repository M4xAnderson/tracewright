# Domain docs

How the engineering skills should consume Tracewright's domain documentation. Layout: single-context.

## Before exploring, read these

- **`GLOSSARY.md`** at the repo root. Domain terms follow it exactly.
- **`docs/adr/`**: read the ADRs that touch the area you're about to work in. `CLAUDE.md` lists the decisions that are already settled.
- **`docs/specs/decisions.md`** and the spec it points to (`docs/specs/*.md`) when the work touches net semantics, properties, state machines, the scan or the file format.

Later decisions in `GLOSSARY.md`, `docs/adr/` and `docs/specs/decisions.md` win over `docs/research/`.

## File structure

```
/
├── GLOSSARY.md
├── docs/adr/          ← decisions, numbered
├── docs/specs/        ← behavior specs and decisions.md
└── crates/ …          ← code (ADR 0004, 0009)
```

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `GLOSSARY.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal: either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR 0008 (evidence for Proved), but worth reopening because…_

`CLAUDE.md` says settled decisions are not reopened, so such a conflict goes to the user, never into code.
