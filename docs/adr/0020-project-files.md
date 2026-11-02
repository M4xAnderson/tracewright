# Project files: lowercase slug IDs, a suppressions file, a format version per file, and a read-only second window

Amends ADR 0001. The project root also holds `suppressions.toml` (the user's decisions about scan findings) and the top view's layout in `layouts/_landscape.toml`; every file carries its own `format` version and is migrated on read. Element, relationship, view and property IDs are **lowercase**, start with a letter, do not end in `-`, have at most 48 characters and are never a Windows device name, because element IDs are file stems and `Billing.toml` and `billing.toml` are one file on Windows and macOS; names inside a behavior model keep their case, contain no `__` and do not end in `-`; `and`, `or` and `not` are reserved for the predicate lexer. Display names keep any case and never change an ID. A place and a transition (or a state and an event) of one element with the same name is a model problem: the file loads, the editor rejects the gesture, and every scope containing the element is blocked; it is never a load failure. A second window on a project whose journal lock is held opens it **read-only**, so every gesture stays journaled. An element file with a kind the app does not know does not load: it is reported, kept byte for byte, never written, and the scopes it could change are blocked; a project from a newer format version is refused. v1 ships no per-entity git merge driver and New project writes no `.gitattributes`; journals moved aside are all kept.

## Considered Options

- **Case-preserving IDs:** readable, but two elements that differ only in case collide on two of three platforms.
- **A second editing window without a journal:** breaks the rule that every gesture is journaled and lets two windows fight over Save.
- **Refuse or open read-only on an unknown element kind:** stops the user from fixing the rest of the project although per-file isolation already loses nothing.

## Consequences

- The slice 4 acceptance text and every fixture use lowercase element IDs.
- `docs/specs/file-format.md` §4, §10, §13, §18 and `docs/specs/decisions.md` D-12, D-19, D-26 carry the rules.

Amended by ADR 0024.
