# "Never holds the whole pool" ships in slice 1; the vacuity guard also runs next to deadlock-free with a final predicate

Amends ADR 0019. The "never holds the whole pool" template (catalog T5, `pool-share`) ships in **slice 1**, alone and unconditionally. The evidence gate of ADR 0019 ("unless a slice 0 dogfooding design has a timeout or retry on a pool path") is removed, and "never more than" with its "never fills up" preset (catalog T4) stays in slice 4 with no gate. E5 case 1 showed that deadlock-free with a final predicate is Proved on the EBS design with a timeout while one holder can take the whole pool; slice 1 is where the final predicate arrives, so the property that catches that defect arrives with it. Slice 1 therefore also carries what T5 needs and nothing more: parsing and resolving the `pool-share` parameters, its expansion and preconditions, its picker entry, the `never p` decision procedure in both explorers with its replay rule, and its exemption from the "user properties are checked from slice 4" gate (`docs/specs/property-language.md` §8.3). The vacuity guard ("no dead transitions" with exploration counts) runs next to user properties **and** next to deadlock-free whenever the scope has a final predicate for that behavior, because a final predicate can make a deadlock-free Proved vacuous in the same way a user property can. The guard needs "no dead transitions", so it still starts in slice 4.

## Considered Options

- **Keep the evidence gate:** waits for a dogfooding design to rediscover a defect E5 already showed, while slice 1 ships the final predicate that hides it.
- **Move T4 to slice 1 with T5:** T4's "never fills up" preset needs the largest-step computation and its own preconditions; nothing in E5 needs it before slice 4.
- **Run the guard only next to user properties:** leaves the E5 case 1 deadlock-free Proved with a final predicate unguarded although it is the case that motivated the guard.
- **Pull "no dead transitions" into slice 1 so the guard runs earlier:** grows slice 1 by a third property kind and its exhaustive evidence; T5 already catches the E5 case 1 defect in slice 1.

## Consequences

- Slices 1–3 check one template (T5); every other template, custom predicate and "no dead transitions" stays Unknown "property cannot be checked" until slice 4.
- In slices 1–3 deadlock-free with a final predicate and T5 are Proved without a guard next to them; their panels show exploration counts only.
- `docs/specs/property-language.md` §8.2, §8.3, §10.1, §11 and §14.5, `docs/product/property-templates.md` §4, §5, §11, §12, `docs/product/v1-slices.md` slices 1 and 4, and `docs/specs/decisions.md` D-60 and D-61 carry the rules. `docs/specs/net-semantics.md` slice tags (§5 heading, §8.3, §15, V57 note), `docs/specs/file-format.md` §6.8, `GLOSSARY.md` (Vacuity guard) and `docs/ux/flows.md` §4.13 follow (decisions D-60, D-61).
