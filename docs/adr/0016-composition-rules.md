# Binding pairs carry their own direction, asynchronous bindings have set-valued ends, and a combined scope's children are the elements whose parent is the scope element

Amends ADR 0002 and ADR 0010. A relationship holds zero or more **binding pairs**; each pair binds one port of each of the relationship's two elements, in either direction, so one relationship carries a call and its reply (a sync pair's `from` is the caller, an async pair's `from` the sender). The fused transition of a sync pair is `relationship/caller+callee` with the **bare** transition names; no two pairs on one relationship may have the same (caller name, callee name), whatever their direction. An asynchronous pair lowers to one `AsyncLink` whose `from` and `to` are non-empty sorted sets of transitions: a Petri net transition port is a one-element set and a state machine event contributes every transition on it; every sender puts one message into the relationship's generated place and every receiver takes one. A relationship carries at most one asynchronous pair. A transition port has exactly one role: caller, callee, or member of **exactly one** asynchronous binding; never fused and asynchronously bound, never in two asynchronous bindings (the meaning of both cases is on record in the composition vectors, so relaxing them later is additive). In a combined scope every **transition** port of every child must be bound by a relationship whose two elements are children; place ports are never bound. A **child** of a combined scope is an element whose parent is the scope element, so Persons and Software Systems (which have no parent) are never children; a scope with no child that has a behavior model has nothing to check and yields no verdict.

## Considered Options

- **Replies as separate relationships** (ADR 0010's one-direction rule kept): every request/reply becomes a RUN-SYNC-CYCLE finding in the scan and doubles the relationship count.
- **Several `AsyncLink`s per relationship sharing one generated place:** declares one place several times and needs a cross-link equality rule for capacity and `when_full`.
- **Children = every element shown in the view:** the view is layout, and Persons and external systems would then need behavior models to check a Container view.

## Consequences

- The canonical encoding of the flattened net does not change; `net`, `verify` and `recheck` stay untouched when state machines arrive in slice 2.
- `recheck::compose` re-checks the set shape and the duplicate rules; the direction rule needs the Model and is engine-only, covered by golden tests (ADR 0010's cross-check covers composition, not lowering).
- `docs/specs/net-semantics.md` revision 3 §2.2, §6 and `docs/specs/decisions.md` D-06, D-07, D-13, D-16, D-31 carry the rules.
