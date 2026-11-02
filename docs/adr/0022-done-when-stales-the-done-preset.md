# Editing "done when" stales deadlock-free and the "can always get back to: done" preset, and nothing else

Amends ADR 0018. A "can always get back to" template property with the preset **done** (catalog T7 "done") expands at lowering to `always reachable F`, where F is the scope element's final predicate for that behavior (ADR 0019: presets follow edits). Its CheckKey therefore covers F, and editing "done when" changes that key as well as the deadlock-free key of the same behavior. ADR 0018's sentence "never stales 'no capacity exceeded' or a user property" is amended to: editing "done when" changes the keys of the deadlock-free property and of every T7 "done" property of that behavior, and of nothing else. This is not spurious staleness under ADR 0013: both properties read F, so a verdict for the old F is a verdict for a different question. "No capacity exceeded", "no dead transitions", "never p" and every "always reachable" property whose target is not the done preset keep their keys.

## Considered Options

- **Freeze F into the T7 "done" property when it is added:** the property keeps asking about an old end state after the user changes "done when"; ADR 0019 rejects frozen presets.
- **Give T7 "done" a key that excludes F and re-expand on display:** a cached Proved for a different F could be shown as current; ADR 0013 requires that a semantic change changes the key.

## Consequences

- `GLOSSARY.md` (Stale), `CODING_STANDARDS.md` VER-4, `docs/specs/net-semantics.md` §11 and §14.4 (g), `docs/specs/property-language.md` §7 and §14.4 T-12, `docs/product/property-templates.md` T1, T7 and its acceptance criteria, and `docs/specs/decisions.md` D-48 carry the rule. Until slice 4 no T7 property exists, so slice 1 behaves exactly as ADR 0018 states.
