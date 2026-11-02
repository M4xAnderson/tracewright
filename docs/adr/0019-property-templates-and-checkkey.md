# Property templates are stored by kind, version and parameters and expanded at lowering; a predicate is hashed as parsed

A template property is stored as its template kind, a template version and its parameters. Lowering expands it at every check into **exactly one** `NetProperty`, reading capacities, initial counts, the largest step and the final predicate from the current model, so presets such as "never fills up", "its initial marking" and "done" follow edits. A failed precondition ends as Unknown "property cannot be checked" with a named line; a hand-written predicate that `when_full` alone decides gets a warning instead. Changing an expansion rule is a format change with a migration (ADR 0001). The CheckKey hashes the predicate **as parsed**: `p > 50` and `p >= 51` have different keys, and a template shares a key with a hand-written predicate only when their trees are equal, because every rewrite would be unchecked code in the shared lowering path and the only cost of not rewriting is a cache miss (ADR 0013 requires that semantic changes change the key, not that equivalent texts share one). Slice 4 runs the vacuity guard ("no dead transitions" next to user properties, with exploration counts shown); New project seeds the two built-in properties as explicit entries in the files, so what is checked is what is stored. The "holds the whole pool" and "never fills up" templates stay in slice 4 unless a slice 0 dogfooding design has a timeout or retry on a pool path; "never at the same time" is held back until two such questions appear in the dogfooding journal; "can reach p" is not a v1 property kind.

Amended by ADR 0023.

## Considered Options

- **Expanded predicate with a template tag** (research): no migrations, but values freeze and a resized capacity keeps asking the old question.
- **Plain predicate recognized by shape:** the value-dependent presets cannot be stored.
- **Normalize equivalent spellings for the key:** more shared code in the trust path for a cache hit.
- **A template as a group of properties with a summary:** a second verdict aggregation the panel would have to explain.

## Consequences

- Each template and preset has a lowering golden test; the verdict panel renders the checked `NetProperty`, never the parameters.
- `docs/specs/property-language.md`, `docs/product/property-templates.md` and `docs/specs/decisions.md` D-10, D-11, D-20, D-40 carry the rules.
