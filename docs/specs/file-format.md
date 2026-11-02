# File format, version 1

Specification `docs/specs/file-format.md`. Status: **Accepted** (applies `docs/specs/decisions.md`; final for implementation once every spec has applied that file), revision 4, 2026-11-02. Owner: senior backend developer. Revision 3 applied the decisions D-01 to D-59 that touch storage (§17.1 maps each one to its section) and closed every open question of revision 2 (§18). Revision 4 applies spike E2 (D-70, ADR 0024): one layout file per element instead of one per view, so `layouts/_landscape.toml` is gone; every file ends with an empty line; and corrected merge advice (§17.1 lists the sections). It is based on:

- `docs/specs/net-semantics.md` revision 3 (cited as **NS §n**)
- `docs/specs/state-machines.md` revision 3 (**SM §n**)
- `docs/specs/property-language.md` revision 3 (**PL §n**)
- `docs/specs/scan-rules.md` v1 (**SR §n**), read with D-18
- `docs/specs/decisions.md` (**D-nn**) and ADRs 0015–0021 and 0024

How the claims were checked (§19):

- Every literal file in §14 was loaded by a throwaway strict reader.
- The same files were checked by a throwaway explorer written from NS §4, §6, §8, §10.1 and §12.
- Every merge result in §15 was produced with git 2.43.0.
- Every toml_edit claim in §11 was measured with toml_edit 0.25.17.

This spec implements three ADRs:

- ADR 0001 as amended by ADR 0020 and ADR 0024: TOML directory, explicit Save, recovery journal, dangling references reported; lowercase slug IDs, `suppressions.toml`, a format version per file, a read-only second window, an unknown element kind isolated to its file; one layout file per element and an empty last line in every file.
- ADR 0011: after-image journal with revisions and `SaveBegin`/`SaveDone`.

It owns **how things are stored**: the directory layout, the TOML schema of every file, IDs, versioning, the writer, the journal, loading and problem reporting. It does not own what the stored data means; §1 lists which spec owns what. Where this text disagrees with an ADR, NS revision 3 or `decisions.md`, those win and this text is the bug.

"Must", "must not" and "may" bind `crates/format`, `crates/model` and the loader. Terms follow `GLOSSARY.md`; §17.3 lists the terms this spec relies on.

Tags **[S0]**–**[S5]** give the slice that first *uses* a shape. Format 1 contains every shape from the start (ADR 0002: both behavior notations are in the format from day one). So the slice 0 reader already parses, validates and round-trips all of them. The one exception is predicate text: slice 0 stores every predicate and `[final]` string verbatim and never parses it (D-25, §6.8).

## 0. Decisions at a glance

| # | Decision | Section |
|---|---|---|
| F1 | Layout: `arch.toml`, `elements/<id>.toml`, `layouts/<id>.toml`, `suppressions.toml`. The directory is flat; `parent` is a key, not a folder. | §2 |
| F2 | Every file starts with `format = <n>`. Files migrate one at a time and lazily. A project with any newer file does not open. | §3.2, §12 |
| F3 | Strictness is per section. An unknown key or a wrong type at the element level makes the file not load. In a behavior model it makes the behavior `Unloaded` (SM-E10). In a property entry the entry is kept, and only that property is not checked (PL E407). A typo can never quietly change a net. | §3.3 |
| F4 | Element, relationship, view and property IDs follow FMT-3 (`order-api`, `order-api-2`): lowercase, letter-first, no trailing `-`, at most 48 characters, never a Windows device name. Names inside a behavior model keep their case, contain no `__` and do not end in `-` (D-12, ADR 0020). IDs never change on a rename, because every part has a display `name`. The slug generator never reissues an ID that anything still references. | §4 |
| F5 | A relationship lives in its **source** element's file. It holds zero or more binding pairs; each pair has its own `mode` (`sync` or `async`), its own direction and qualified ports, so one relationship carries a call and its reply (D-07, ADR 0016). The scan's `interaction` and `timeout` are separate keys, and bindings never set them. | §6.7 |
| F6 | Ports: a Petri net place or transition is a port with `port = true`. In a state machine, input and output events are transition ports, states are place ports, and internal events are not ports (ADR 0015). Only transition ports are bound (NS §6.2). | §6.4–6.6 |
| F7 | Scan attributes follow SR §2.3–2.6 exactly: `replicas`, `zones`, `service`, `interaction`, `timeout`. An absent key means **not stated**. | §6.3, §6.7 |
| F8 | Properties are `[[property]]` entries in the scope element's file, in the shape of PL §9, built-ins included; templates are stored by kind, version and parameters (D-11). Final predicates are `[final]` with the keys `combined` and `own`; lowering copies each into the deadlock-free property of its behavior (D-04). Predicate strings are opaque in slice 0, `[final]` is parsed from slice 1 and properties from slice 4 (D-25). | §6.8 |
| F9 | Each element's layout is in its own file `layouts/<id>.toml`: its position in its parent's view (the landscape for a top-level element), the bend points of its outgoing relationships and its behavior-model layout (ADR 0024). A view is drawn from its children's files. A missing entry means unplaced. A stale entry is kept and reported. | §7 |
| F10 | Suppressions are a `[[suppression]]` array in one project file, keyed by the finding key of SR §3.7. | §8 |
| F11 | Entities (element, behavior model, relationship, property, final predicate, layout entry, suppression) are the unit of Save diffs, journal after-images and Reload conflicts. Array entries are identified by their `id` or `key`, never by position. | §9 |
| F12 | Writer: new text is canonical, and every file ends with an empty line (ADR 0024). Existing files are edited in place with `toml_edit`: untouched bytes stay identical, value comments survive because decor is copied, and line endings are restored per file. | §11 |
| F13 | Journal: a framed append-only log (`@record <len> <hash>` + TOML body) with `seq`, `rev`, expected file hashes and `save_begin`/`save_done`. | §13 |
| F14 | Never check a smaller net: a file or section that did not load blocks exactly the scopes it could change. | §10.5 |
| F15 | LIM-2 limits on bytes and counts are checked before the model is built. Exceeding one gives a named problem, never a truncation. | §3.5 |

## 1. Ownership: what this spec defines and what it cites

| Topic | Normative text | This spec |
|---|---|---|
| Qualified IDs, one namespace per element, the slug facts the semantics need | NS §2.2 | narrows the slug alphabet for file-system reasons (§4.1; D-12, ADR 0020) |
| Ports, bindings with their own direction, children of a combined scope, lowering rules L1–L16, assumptions, `nothingToCheck` | NS §6, ADR 0015, ADR 0016 | stores bindings (§6.7) and maps each stored defect to its L-rule (§10.3) |
| Capacity, `when_full`, the default for generated places | ADR 0014, NS §6.4 | stores them; places of a behavior model get no default |
| State machine content, ports, codes SM-E01–E12 and SM-B01–B07, limits | SM §1, §2, §8 with D-07, D-14, D-42 | chooses the TOML syntax (§6.5) |
| Predicate grammar and normalization; which slice parses it | NS §5.3, PL §3.2; D-25 | stores the text verbatim; defines no second grammar |
| Property entries, field rules, writer order, codes E1xx–E4xx; templates | PL §9, §10; ADR 0019 | adopts them verbatim (§6.8) |
| Final predicate: home, slice, defaults | NS §3.3, §8.1, ADR 0018 | stores `[final]` (§6.8) |
| Which places a combined-scope predicate may name | NS L9, ADR 0015 (D-01) | validates against it (§6.8, §10.3) |
| Scan attributes, their values, "not stated" | SR §2, ADR 0021 | adopts the encoding of SR §2.6 (§6.3, §6.7) |
| Suppressions and finding keys | SR §2.7, §3.7, §3.8 (D-18) | adopts them (§8) |
| Project file list, slug alphabet, second window, unknown element kind; one layout file per element | ADR 0020, ADR 0024 | details them (§2, §4, §10.5, §13.1) |
| Directory layout, file schemas, IDs and renames, references, versioning, writer, journal, loading and problem reporting, limits, merge behavior | **this spec** | — |

## 2. Project directory

### 2.1 Layout

```text
<project>/                          the folder the user opens; usually inside a git repository
  arch.toml                         project file; its presence makes the folder a project
  elements/<element-id>.toml        one file per element: kind, attributes, behavior model, outgoing relationships, properties, final predicate
  layouts/<element-id>.toml         one file per element: its position and size in its parent's view (the landscape when it has no parent),
                                    the bend points of its outgoing relationships, and its behavior-model layout
  suppressions.toml                 suppressions; created on the first suppression [S5]
```

- **Project files** are exactly `arch.toml`, `elements/*.toml`, `layouts/*.toml` and `suppressions.toml`. Nothing else is read, written, hashed or journaled. Other files (a README, `.git/`, subfolders) are ignored.
  - A `*.toml` file in `elements/` or `layouts/` whose stem is not a valid ID (§4.1) gets the problem `schema` "file name is not a valid ID" and is not loaded. That includes `layouts/_landscape.toml`, which revision 3 used and ADR 0024 removed.
- **Dotfiles** in `elements/` and `layouts/` are ignored. The app's own temporary files are named `.pn-<8 lowercase hex>.tmp` (§11.4). At open, the app deletes leftovers that match this exact pattern, and nothing else.
- **Flat:** an element file stays put when the element moves to another parent. Re-parenting edits one key and never moves a file in git.
- **Path budget** (Windows `MAX_PATH` = 260, including the terminating null):
  - Element IDs have at most 48 characters, so the longest project file path is the project path + `\elements\` (10) + 48 + `.toml` (5).
  - A project path of up to 196 characters keeps that at 259 or less.
  - Temporary names (15 characters) are shorter than any target name, so a temporary path is never longer than its target's.
  - Longer project paths depend on Windows long-path support, which the Windows build checks (E3).
  - A write that fails for path length is a failed write (§11.4), never a silent truncation.

### 2.2 Why this split

ADR 0001 sets one file per element, with layout kept apart; ADR 0024 gives layout one file per element too, so every entity has exactly one owning element. Everything else goes where each common edit touches one file:

- a drag of one element touches that element's layout file, and dragging two elements touches two different files;
- a net edit, a binding or a property touches one element file;
- a display rename touches one element file;
- adding an element to a view adds two files (`elements/<id>.toml` and `layouts/<id>.toml`) and edits no shared file.

§15 measures the result on real merges. Revision 3 kept one layout file per view; spike E2 found that two elements added to one view then conflict in that shared file (§15 S6b), which is why ADR 0024 changed the split.

### 2.3 Sizes

At the target sizes (300 elements per view, 2,000 per project) a project has:

- about 2,000 element files of 0.3–5 KB;
- at most one layout file per element (every placed element has one), so up to about 2,000 layout files;
- one suppressions file.

That is up to about **4,000 files**. The open-time target is **[assumption]** under 1 s for 4,000 files on an SSD. Slice 0 measures it on a generated project of 2,000 elements with a layout file each (`docs/product/v1-slices.md`, slice 0); it is a dev measurement, not a user-facing criterion.

### 2.4 New project [S0, contract R16]

In an empty folder, "New project" writes `elements/<slug(folder name)>.toml` first and `arch.toml` last. `arch.toml` is what makes the folder a project, so a crash between the two writes never leaves a project without its system.

The system file holds `kind = "software_system"`, `name` = the folder name, and the two built-in property entries that PL §9.2 seeds (D-20, ADR 0019):

```toml
format = 1
kind = "software_system"
name = "kinesis-incident"

[[property]]
id = "deadlock-free"
behavior = "combined"
kind = "deadlock_free"

[[property]]
id = "no-capacity-exceeded"
behavior = "combined"
kind = "no_capacity_exceeded"
```

- No layout file is written, so the system starts unplaced.
- The app writes no `.gitignore` or `.gitattributes` (D-19, ADR 0020).
- The slug of the folder name follows §4.3; the two property IDs are fixed, because the project is empty.
- Until the system has children with behavior models, both entries have the key state `nothingToCheck` (NS §6.6, D-13).

## 3. Rules for every file

### 3.1 Encoding and syntax

- UTF-8. A leading BOM is accepted and stripped on read, and never written. Line endings: §11.3.
- The reader accepts TOML 1.1 (toml_edit 0.25.17 implements spec 1.1.0).
- **Text the app generates** (new files, new entities, replaced values) uses only syntax that is also valid TOML 1.0:
  - no newlines and no trailing commas in inline tables;
  - no `\e` or `\xHH` escapes;
  - times with seconds.
- In-place editing (§11.2) keeps any TOML 1.1 syntax the user wrote in untouched parts, so a hand-edited file may need a TOML 1.1 reader.
- No floats appear anywhere in the schema. Every number is an integer, which keeps formatting and hashing deterministic.

### 3.2 The `format` key

Every project file has the top-level integer key `format`, and the canonical writer puts it first. This spec describes format 1.

- A missing `format` is the problem `schema` "format is missing; add `format = 1`", and the file does not load.
- A file whose `format` is newer than the app supports stops the whole project from opening (§12.2).

### 3.3 Strictness, per section

Three things are reported: a key that this version does not define, a wrong TOML type, and a missing required key. Each report gives the file, the line and, where one exists, the closest known key ("unknown key `when_ful` in place `queue`; did you mean `when_full`?"). What fails depends on where the defect is:

| Where | Effect | Why |
|---|---|---|
| Anywhere in `arch.toml` | The project does not open. | The format and budget of the project would be unknown. |
| Element level of an element file: top-level keys, `[relationships.*]` and their binding pairs, `[final]` | The **file does not load** (`schema`). Its bytes are kept and never written, and the blocking rule of §10.5 applies. | A dropped relationship or binding could remove a fused-transition alternative and make a net smaller, which can turn a Disproved into a Proved. |
| Inside `[petri_net]` or `[state_machine]`, including enum values (`when_full`, `direction`) and the per-net limits | The element loads with `BehaviorModel::Unloaded` (SM §1.4, SM-E10, SM-E12) and the problem `behaviorNotLoaded`. Every scope that contains the element fails L1. The section's bytes are kept and never written. Behavior edits on it are Rejected until the file is fixed and reloaded. | The rest of the element (relationships, the properties of its scopes, scan attributes) stays usable, and no scope is checked without that behavior. |
| Inside a `[[property]]` entry | The entry is kept verbatim and reported with its PL code (E407 for an unknown key, E412 for a bad template parameter, …). Only that property is not checked (PL §9.2). | A property is one question; nothing else depends on it. |
| A predicate string (`predicate`, a `[final]` value, a template parameter) | Never a schema problem. Slice 0 stores it verbatim and never parses it; parse and resolution errors (E1xx, E2xx) are reported from slice 1 for `[final]` and from slice 4 for properties and template parameters (D-25, §6.8). | The string is the user's text; one parser (`model::property`) judges it, in the slice that checks it. |
| A `layouts/*.toml` file | The file does not load. That element's position is unplaced, its outgoing relationships are routed automatically and its behavior layout is unplaced; nothing else is affected. | Layout never changes meaning. |
| `suppressions.toml` | The file does not load, and every finding shows unsuppressed. | Failing open shows more, never less. |

**Model problems, not schema problems.** Some missing data is what NS calls "model incomplete": `when_full`, `capacity`, a behavior model, an async capacity, a binding. That is not a schema error. The file loads, and validation reports the gap with its L-rule (§10.3), so the user can see the incomplete net. Scan attribute values outside their range are model problems too (SR §2.6, §3.10).

### 3.4 Value ranges

| Value | Type | Range | Outside the range |
|---|---|---|---|
| place `capacity`, arc weight, async binding `capacity` | integer | 1 ..= 2,147,483,647 | Below 1 or above the range: behavior `Unloaded` (schema). |
| place `initial` | integer | 0 ..= 2,147,483,647 | Negative or above the range: `Unloaded`. Above the place's capacity: model problem `badInitial` (L10; NS V29 expects the file to load). |
| `replicas`, `zones` | integer | 1 ..= 1,000 | model problem `badAttribute`; the scan treats the value as not stated (SR §3.10) |
| `service` | string | 1–64 characters after trimming | empty: `badAttribute`; longer than 64: `schema` (SR §2.6, LIM-2) |
| layout `x`, `y`, bend points | integer | −1,000,000 ..= 1,000,000 | the layout file does not load |
| layout `w`, `h` | integer | 1 ..= 1,000,000 | the layout file does not load |
| `name` (element, relationship, place, transition, state, event), `meaning`, `technology` | string | 1–200 characters, one line, no control characters | `schema` at its level |
| `description` | string | up to 10,000 characters; newlines allowed | `schema` at its level |
| property `label`, `predicate`, `[final]` values | string | PL §9.2: `label` ≤ 200 characters; a predicate ≤ 4,096 bytes (E114) | PL codes. The byte length is checked in every slice, before any parsing, so slice 0 also reports E114. |
| suppression `reason` | string | 1–500 characters after trimming | model problem `badSuppression` (SR §2.7) |
| suppression `date` | TOML local date | `YYYY-MM-DD` | the suppressions file does not load |

**Why capacities and weights stop at 2³¹ − 1.** This is a deliberate narrowing of NS §2.3 rule 6, for two reasons:

- Predicate constants and template expansions are i32 (PL E308), so a predicate could not name a capacity above that.
- NS §2.3 rule 7 becomes unreachable from files: cap + weight ≤ 2³² − 2 < u32::MAX.

NS vectors V51–V54 are therefore net-level vectors only and cannot be written as files.

### 3.5 Limits (LIM-2)

- A file's byte limit is checked on its metadata, and again on the bytes read, **before** the TOML parser sees them. This bounds the parser's own allocation.
- Every count limit is checked on the parsed document's table and array lengths **before** any `Model` value is built.
- Limits are named constants in `crates/format`, with headroom above the targets (300 elements per view, 2,000 per project).
- Exceeding a limit is a problem with a message, never a panic and never a truncation.

| Constant | Value | Checked | Exceeded |
|---|---|---|---|
| `ARCH_FILE_BYTES_MAX` | 64 KiB | before parse | the project does not open |
| `ELEMENT_FILE_BYTES_MAX` | 4 MiB | before parse | `tooLarge`; the file does not load |
| `LAYOUT_FILE_BYTES_MAX` | 4 MiB | before parse | `tooLarge`; the layout file does not load |
| `SUPPRESSIONS_FILE_BYTES_MAX` | 4 MiB (SR §2.7) | before parse | `tooLarge`; the file does not load |
| `ELEMENTS_MAX` | 10,000 per project | on the directory listing of `elements/`, before any file is read | the project does not open ("more than 10,000 elements") |
| `RELATIONSHIPS_MAX` | 40,000 per project; 1,024 per element file | per file before decoding; the project total while loading | per file: the file does not load; total: the project does not open |
| `BINDINGS_PER_RELATIONSHIP_MAX` | 64 | per relationship | the file does not load |
| `PN_PLACES_MAX`, `PN_TRANSITIONS_MAX` | 4,096 each per Petri net | before decoding the behavior | `behaviorNotLoaded` |
| `PN_ARCS_MAX` | 1,024 per transition (pre + post), 65,536 per net | before decoding the behavior | `behaviorNotLoaded` |
| `SM_STATES_MAX`, `SM_EVENTS_MAX`, `SM_TRANSITIONS_MAX` | 1,024, 1,024, 4,096 (SM §1.4) | before decoding the behavior | `behaviorNotLoaded` (SM-E12) |
| properties | 256 per element, 4,096 per project (PL §9.2) | per file before decoding; the project total while loading | E409. All entries are kept. Above the per-element limit, none of that element's properties is checked. Above the project limit, none is checked in the file that crosses it or in any file after it. |
| `SUPPRESSIONS_MAX` | 10,000 (SR §2.7) | before decoding | `tooLarge`; the file does not load |
| suppression `key` | 131,072 bytes | before decoding the entry | `badSuppression` |
| `LAYOUT_ENTRIES_MAX` | 20,000 per layout file (the position, relationship entries and behavior items together); 64 bend points per relationship | before decoding | the layout file does not load |
| `JOURNAL_RECORD_BYTES_MAX`, `JOURNAL_BYTES_MAX` | 16 MiB per record body, 256 MiB per journal | before writing a record and before reading one | writer: journaling stops for the session with the warning "Save to keep crash recovery" (Save still works); reader: the frame counts as damaged (§13.2) |

## 4. IDs

### 4.1 Alphabets

There are two alphabets. Both lie within NS §2.2 (`[A-Za-z_][A-Za-z0-9_-]*`, never `and`, `or` or `not`) and within FMT-3.

| Alphabet | Pattern | Used for |
|---|---|---|
| **ID** | `^[a-z](?:[a-z0-9_-]{0,46}[a-z0-9_])?$` | element IDs (file stems), relationship IDs, view IDs, property IDs |
| **Name** | `^[A-Za-z_](?:[A-Za-z0-9_-]{0,46}[A-Za-z0-9_])?$` and no `__` | places, transitions, states, events, state machine transitions |

Both alphabets allow 1–48 characters and exclude `and`, `or` and `not`. IDs also exclude the Windows device names `con`, `prn`, `aux`, `nul`, `com1`–`com9` and `lpt1`–`lpt9`, which cannot be file names on Windows, even with an extension.

These narrow NS §2.2, which allows the narrowing; they are decided (D-12, ADR 0020) and NS §2.2 revision 3 cites them. Each has a stated reason:

- **IDs are lowercase only.** Element IDs are file stems, and `Billing.toml` and `billing.toml` are one file on Windows and macOS. Relationship IDs share the element namespace (§4.2), so they follow the same rule. Display names keep any case.
- **IDs start with a letter** (D-12), so a file stem that starts with `_` or a digit is never an element. Revision 3 relied on this for `layouts/_landscape.toml`; ADR 0024 removed that file, and the rule stays as decided.
- **Neither ends in `-`.** Otherwise the predicate lexer would read a following minus as part of the ID (`x/p- 1`, PL E111).
- **Names contain no `__`.** SM §1.3 F1 reserves it for later per-leaf names. The rule applies to both behavior notations, so the editor has one rule. IDs need no such rule: the slug generator never produces `__` (§4.3), and nothing is reserved in the ID namespace.
- **At most 48 characters**, for the path budget of §2.1.

`|`, `=` and `,` cannot occur, so finding keys need no escaping (SR §2.7). `/`, `+` and whitespace cannot occur either, so qualified IDs stay unambiguous (NS §2.2).

Consequences recorded by ADR 0020: every fixture and the slice 4 acceptance text use lowercase element IDs (`never billing/inflight + shipping/inflight > 2`); the UI still shows display names in any case. A hand-written file with an upper-case element ID has an invalid file stem and is the problem `schema` "file name is not a valid ID" (§2.1).

### 4.2 Namespaces

| What | Unique within | Stored as | Global form |
|---|---|---|---|
| Element | project, **shared with relationships** | file stem `elements/<id>.toml` | `<id>` |
| Relationship | project, **shared with elements** | table key `[relationships.<id>]` in the source element's file | `<id>`; it is also the generated place's qualified ID |
| View | project | not stored: a view is named by the element whose inside it shows, and is drawn from the layout files of that element's children. The landscape (elements without a parent) has no ID in the files. | `<view>` |
| Place, transition (Petri net) | element; one namespace for both (NS §2.2) | `[petri_net.places.<name>]`, `[petri_net.transitions.<name>]` | `element/name` |
| State, event, state machine transition | element; one namespace for all three (SM-E04) | `[state_machine.states.<name>]`, `[state_machine.events.<name>]`, `[state_machine.transitions.<name>]` | `element/name` |
| Fused transition | flattened net | not stored | `relationship/caller+callee`, with bare names (NS §2.2) |
| Property | project (PL §9.2) | `id` of a `[[property]]` entry | `<id>` |
| Suppression | project | `key` of a `[[suppression]]` entry | the finding key (SR §3.7) |

Elements and relationships share one namespace for two reasons: a bare ID in a predicate can only mean a generated place, and a finding key can mix element and relationship IDs without ambiguity. Property IDs have their own namespace (PL §9.2).

### 4.3 Deriving an ID from a name

```text
slug(name, fallback):
  s = NFKD(name) with combining marks (Unicode category Mn) removed
  s = lowercase(s)
  every character outside [a-z0-9_-] becomes "-"
  collapse runs of "-" into one "-", and runs of "_" into one "_"
  trim "-" and "_" at both ends
  if s is empty:            s = fallback
  if s starts with a digit: s = fallback + "-" + s
  cut s to 40 bytes; trim a trailing "-" or "_"
allocate(base, taken):
  if base is not taken and not reserved: return base
  return base + "-" + n for the smallest n >= 2 such that the result is not taken
```

Examples:

- "Order API" → `order-api`; a second "Order API" → `order-api-2` (FMT-3).
- "Front-end server fe1" → `front-end-server-fe1`.
- "Café" → `cafe`.
- "3rd party" → `element-3rd-party`.
- "And" → `and-2` (`and` is reserved).

The 40-byte cut leaves room for the suffix within 48 characters. Fallbacks are `element`, `relationship`, `place`, `transition`, `state`, `event` and `property`. A relationship's base is `<source>-<target>`. A property's base is its kind or template ID (`deadlock-free`, `pool-share`; PL §9). The generator always produces lowercase; the reader accepts the full Name alphabet in behavior models.

**Taken.** The generator never issues an ID or name that the project still uses **or still references** (PL §4.5, A10; vector PL E-32). A dangling reference keeps its target ID out of use until the user edits or deletes the reference. Otherwise a new place could silently become the subject of an old property.

| Generating | Taken means |
|---|---|
| Element or relationship ID | every element and relationship ID in the current model and in the saved snapshot; the stem of **every** file in `elements/` and `layouts/`, including files that did not load; every relationship key the loader saw leniently (§10.5) in an element file that did not load; every ID **referenced** anywhere, whether it resolves or not: `parent`, relationship `to`, the element part of binding ports, the element part of qualified IDs and the bare relationship IDs in predicates, final predicates and template parameters, the stems of layout files and the relationship keys in layout files (orphans included), and every ID in a suppression key |
| Name inside element E | every name in E's behavior model (for an `Unloaded` behavior, every table key the loader saw); every `E/<name>` referenced anywhere: binding ports, predicates, final predicates, template parameters, E's behavior layout entries (orphans included) |
| Property ID | every property ID in the project, including entries with problems |

### 4.4 Stability and rename semantics

IDs are created once and never change on a rename (FMT-3). Every part that the UI names has a display `name`, so a rename never changes an ID, a reference or a CheckKey (ADR 0013).

| Change | What happens in the files | Effect on verdicts |
|---|---|---|
| Rename an element, relationship, place, transition, state, event or state machine transition | Only `name` changes (`meaning` for a relationship). | None: names are not part of the CheckKey (ADR 0013, NS §14.4 d, SM K-04). |
| Rename a property | Only `label` changes (PL §9.2). | None |
| Change an ID by hand (rename the file, or edit a table key or an `id`) | The old ID disappears and a new one appears. Every reference to the old ID is reported as dangling; the app does not guess. | Affected scopes fail lowering (L3) until the references are fixed. |
| Re-parent an element | `parent` changes. `[position]` is removed from the element's own layout file, so the element is unplaced in its new view. The bend points of its relationships whose ends stop being siblings are removed from their source's layout file. No entry moves between files. | The combined scopes of the old and new parent change. |
| Reconnect a relationship's source | The `[relationships.<id>]` table moves from the old source's element file to the new one; the ID stays. Binding pairs that named a port of the old source are removed. Its bend points are removed from the old source's layout file, so it is routed automatically. | The affected scopes change. |
| Reconnect a relationship's target | `to` changes. Binding pairs that named a port of the old target are removed. Its bend points are removed from the source's layout file, so it is routed automatically. | The affected scopes change. |

An ID derived from a placeholder name ("Container 7" → `container-7`) keeps it. A "Change ID…" command that rewrites every reference as one Batch could be added later; it is not part of v1.

Revision 1's rule "the ID follows the name until the first Save" is withdrawn. It contradicted FMT-3 and complicated journal images, undo snapshots and CheckKeys.

## 5. Project file `arch.toml`

```toml
format = 1

[budget]                 # optional [S1]; when absent, the engine's defaults apply
time_ms = 2000
max_markings = 2000000
```

| Key | Type | Meaning |
|---|---|---|
| `format` | integer, required | Format version of this file |
| `budget.time_ms`, `budget.max_markings` | integer, 1 ..= 2,147,483,647, optional | Budget per check (glossary: Budget). It lives in the project so that the app and a later CLI get the same Unknowns. It is **not** part of any CheckKey (ADR 0013, NS §11), so changing it never makes a Proved or Disproved stale. |

There is no per-project rule disabling (SR §2.7, §11), so revision 1's `[scan] disabled_rules` is withdrawn. The UI shows the folder name as the project name; the system's name is in its element file.

## 6. Element files `elements/<id>.toml`

### 6.1 Top-level keys

| Key | Type | Required | Meaning |
|---|---|---|---|
| `format` | integer | yes | §3.2 |
| `kind` | string | yes | Element kind (§6.2). An unknown kind is `schema`: only this file does not load; it is reported, kept byte for byte, never written, and the scopes it could change are blocked (§10.5; D-19, ADR 0020). The rest of the project stays editable. |
| `name` | string | no; defaults to the ID | Display name. The writer always writes it. |
| `parent` | ID | no | The element whose view shows this element. When absent, the element is top-level and shown in the landscape. An element is a **child** of the combined scope of its `parent` (NS §6.6, D-13). |
| `technology`, `description` | string | no | Display only |
| `replicas`, `zones`, `service` | §6.3 | no | Scan attributes |
| `[petri_net]` or `[state_machine]` | table | no; at most one | The behavior model (§6.4, §6.5). Both present: `schema` at the element level. |
| `[relationships.<id>]` | tables | no | Outgoing relationships with their binding pairs (§6.7) |
| `[[property]]` | array of tables | no | Properties whose scope is this element (§6.8) |
| `[final]` | table | no | Final predicates of this element's scopes (§6.8) |

### 6.2 Kinds and levels

Format 1 kinds: `person`, `software_system`, `container`, `database`, `queue`, `component`, `code` (SR §2.1). All seven are in the format from slice 0 (SR §2.1 requirement 1): the canvas needs `kind` for the icon, and adding kinds later would need a format bump. Slice 0 offers Container, Database and Queue in the UI.

The **level** is derived, not stored: depth 0 is System Context, 1 Container, 2 Component, 3 Code.

A kind that does not match its level, or a parent of the wrong kind, is the model problem `kindLevel` (SR §2.1 table, §3.10). The allowed parents are:

- Person and Software System: no parent.
- Container, Database and Queue: a Software System.
- Component: a Container.
- Code: a Component.

`kindLevel` excludes the element, and everything below it, from every scan rule. It does not block checks, because the kind is not part of the net (NS §11).

**Children and kinds (D-13).** A child of a combined scope is an element whose `parent` is the scope element. Persons and Software Systems have no parent, so in a valid project they are never children; a Person in a design that needs behavior is modeled as a container stand-in (§14). A hand-written Person or Software System that does carry a `parent` gets `kindLevel` and **is still a child** of that parent's combined scope: lowering reads `parent`, never `kind`, so it fails L1 there when it has no behavior model and another child has one. Dropping it would check a smaller net than the files describe, so the conservative reading is the rule.

### 6.3 Scan attributes [S5]

An attribute is data the scan reads. It never changes a behavior model and is never part of a CheckKey; SR §3.9 hashes it into the scan input hash instead.

**Not stated is an absent key.** There is no third spelling such as `"unknown"`, and there are no kind defaults (SR §2.2). The reasons:

- one meaning gets one encoding;
- a not-stated attribute costs no diff line;
- setting an attribute back to not stated deletes the key.

Projects from earlier slices therefore load unchanged.

| Key | On | Value | Allowed kinds | Read by |
|---|---|---|---|---|
| `replicas` | element | integer 1 ..= 1,000 | `container`, `database`, `queue` | REL-SPOF |
| `zones` | element | integer 1 ..= 1,000; `zones ≤ replicas` when both are stated | `container`, `database`, `queue` | REL-SPOF |
| `service` | element | string, 1–64 characters after trimming, compared exactly | `container`, `database` | DATA-SHARED-DB |
| `interaction` | relationship | `"synchronous"` or `"asynchronous"` | — | RUN-SYNC-CYCLE, REL-NO-TIMEOUT, REL-SPOF |
| `timeout` | relationship | `"yes"` or `"no"` | — | REL-NO-TIMEOUT |

- A wrong TOML type (a string for `replicas`) or a `service` longer than 64 characters is `schema`.
- Every other wrong value is the warning `badAttribute`: out of range, `zones > replicas`, an empty `service`, an unknown `interaction` or `timeout` value, or an attribute on a kind that does not allow it. The value is kept in the file and shown in the problems panel, and the scan treats it as not stated (SR §3.10).
- Revision 1's `owner`, `entry_point` and `logical_db` are withdrawn (SR §2.3, §11 item 8). Entry points are relationships from a Person.

### 6.4 Petri net

```toml
[petri_net.places.peer_threads]
name = "peer threads"        # optional display name; defaults to the key
capacity = 4                 # required; model problem noCapacity when missing (L10)
when_full = "overflow"       # required: "block" | "drop" | "overflow"; model problem noWhenFull when missing (L2)
initial = 0                  # optional, default 0; the writer omits it when 0
port = false                 # optional, default false; the writer omits it when false

[petri_net.transitions.receive_info]
port = true                  # optional, default false
pre = { peer_threads = 1 }   # place name -> arc weight; optional, default empty
post = { map_entries = 1, peer_threads = 1 }
```

- `pre` and `post` follow ADR 0010 and NS §2.1. A read arc is written as the same place in `pre` and `post`, with equal weights.
- A place of a behavior model has **no default** for `when_full` (ADR 0014). Only generated places default to `overflow`.
- An arc to a place that does not exist in the same net is `dangling` (L3). A weight of 0 is a schema error inside the behavior (`behaviorNotLoaded`).
- Places and transitions share one namespace (NS §2.2). The same key under `places` and `transitions` is the model problem `nameClash` (L10: "billing has both a place and a transition named `ret_a`").
- The initial marking is the set of `initial` values. Final predicates are scope data (§6.8).

### 6.5 State machine [S2]

The content is SM §1.1; the syntax is chosen here, as SM §1.5 leaves it to this spec. SM §1.5 shows inline tables. This spec uses multi-line tables instead, so that each state, event and transition is its own block in a diff and concurrent edits merge (§15).

```toml
[state_machine]
initial = "closed"           # required; must name a state (SM-E02, SM-E03)
final = ["closed"]           # optional list of state names; default empty; the writer omits it when empty (SM-E09)

[state_machine.states.closed]
name = "Closed"              # optional; defaults to the key

[state_machine.states.open]

[state_machine.events.failure]
direction = "internal"       # required: "input" | "output" | "internal" (SM-E10 for any other value)

[state_machine.events.request]
direction = "input"

[state_machine.transitions.pass]
from = "closed"              # required (SM-E06)
event = "request"            # required; must be a declared event (SM-E07)
to = "closed"                # required (SM-E06)

[state_machine.transitions.trip]
from = "closed"
event = "failure"
to = "open"
```

- States, events and transitions accept only `name` plus the keys shown. These make the behavior `Unloaded` (SM-E10, F6, F9):
  - any other key anywhere in `[state_machine]` (`guard`, `parent`, `entry`, `effect`, `regions`, …);
  - a missing `direction`;
  - a `direction` outside the three values.
- Exceeding the SM §1.4 limits is SM-E12. It is checked on the table lengths before decoding (§3.5).
- States, events and transitions share one namespace (SM-E04). The same key in two of the three tables is a model problem (L14), not a schema error, so the user sees both entries.
- Events are declared, never inferred from transitions. This is what lets the loader report SM-E07 (undeclared event) and SM-E11 (an input or output event used by no transition).
- The format stores states, events and transitions, never the compiled net (slice 2 acceptance criterion). A state has no `capacity` or `when_full` key: lowering compiles every state to a place of capacity 1 with `when_full = overflow` (SM §4.1, D-14), so a scope with a state machine never gets the `NoOverflowPlace` certificate.
- `final` is stored data in every slice. It never writes anything into `[final]`: from slice 2 lowering derives the own-scope default `Σ E/f = 1` over the final states when the element's `[final]` has no `own` key (D-42); a stated `own` replaces the default.
- SM §1.5 (revision 3) shows the same content in this syntax (`[state_machine]`, multi-line tables); the stored syntax is defined here (§17.2).

### 6.6 Ports

| Behavior | Transition ports (bound by relationships) | Place ports (never bound; nameable by a combined-scope predicate under NS L9) | Not ports |
|---|---|---|---|
| Petri net | transitions with `port = true` | places with `port = true` | everything else |
| State machine | every `input` and `output` event, standing for every transition on it (SM §2.1, D-06) | every state, implicitly (ADR 0015, D-01) | `internal` events; the state machine transitions themselves |

There is no separate port list: a port cannot exist without what it exposes, so nothing can drift.

- In a combined scope, every transition port and every input or output event must be bound by a contributing relationship (L4). A relationship contributes iff both its elements are children of the scope (NS §6.6).
- Place ports are never bound (NS §6.2, ADR 0016).
- A binding pair that names a place, a place port, a state, an internal event, or a transition without `port = true` is `notAPort` (L5).

### 6.7 Relationships and their binding pairs

```toml
[relationships.membership-updates]
to = "fe1"                       # required: the target element; the source is this file's element
meaning = "publishes peers to"   # optional display text
technology = "gRPC"              # optional display text
interaction = "asynchronous"     # optional scan attribute (§6.3); bindings never set it
timeout = "no"                   # optional scan attribute (§6.3)

[[relationships.membership-updates.binding]]
mode = "async"                   # required: "sync" | "async"
from = "membership/publish"      # required: qualified port; sync: the caller, async: the sender
to = "fe1/open_peer_thread"      # required: qualified port; sync: the callee, async: the receiver
capacity = 8                     # async: required (L13 when missing); sync: not allowed
when_full = "overflow"           # async: optional, default "overflow" (ADR 0014), always written; sync: not allowed
```

A relationship holds **zero or more binding pairs** (glossary **Binding**: one pair, with its own direction). NS C3 has two fusions on one relationship, SM §2.2 allows several pairs, and a call and its reply are two pairs of one relationship (D-07, ADR 0016, SR §2.5).

- A sync pair is one `Fusion`: `from` is the caller, `to` the callee. Its fused transition is `<relationship>/<bare from name>+<bare to name>` (NS §2.2).
- An async pair is the relationship's one `AsyncLink`: `from` is the sender, `to` the receiver. For a state machine event, the port stands for all of the event's transitions, so `AsyncLink.from` and `AsyncLink.to` are sets (D-06).
- **Direction is per pair.** `from` and `to` name ports of the relationship's two elements, one each, **either way around** (NS L11). The drawn direction of the relationship (source → target) is the scan's dependency direction and does not constrain the pairs.
- Ports are written as **qualified** IDs (`element/name`), the same text that predicates, problems and evidence use, so the element of each end is explicit in the file.
- **Binding mode is not interaction** (D-29). `mode = "sync" | "async"` is per pair and decides the net; `interaction = "synchronous" | "asynchronous"` is a relationship attribute the scan reads and the user states.

A call and its reply on one relationship (E5 case 1, `r1` of NS §13.1):

```toml
[relationships.r1]                   # in elements/api_a.toml: api_a calls control_plane
to = "control_plane"
interaction = "synchronous"

[[relationships.r1.binding]]
mode = "sync"
from = "api_a/call"                  # the call: caller in the source; fused transition r1/call+accept_a
to = "control_plane/accept_a"

[[relationships.r1.binding]]
mode = "sync"
from = "control_plane/return_a"      # the reply: caller in the target; fused transition r1/return_a+reply
to = "api_a/reply"
```

**Rules.** Each defect below is a model problem, so the file still loads. §10.3 maps each one to its L-rule or SM code.

| Rule | Problem | Rule source |
|---|---|---|
| `from` and `to` resolve to an existing port | `unknownPort` (does not exist), `notAPort` (exists, but is neither a transition port nor a port event) | L3, L5, SM-B02 |
| `from` and `to` are ports of the relationship's two elements, one in each, in either order; an output event is used only as `from` and an input event only as `to`, at either end | `bindingDirection` | L11, SM-B01 (D-07) |
| the two ports belong to two distinct elements | `bindingSelf` | L6, SM-B06 |
| a relationship has at most one `async` pair | `bindingAsync` | L8 |
| no two sync pairs on one relationship have the same (bare `from` name, bare `to` name), **whatever their direction**: `api/call → billing/serve` and `billing/call → api/serve` would both fuse to `r/call+serve`. This is the Petri net case of NS L12 and a necessary check for state machines; the complete check runs on the fused IDs, whose bare names are state machine transition names for an event pair (SM §2.2, V-25; D-50), in `engine::lower` and again in `recheck::compose` | `bindingDuplicate` | L12 |
| a sync pair has no `capacity` or `when_full`; an async pair has a `capacity` | `badBinding`; `noCapacity` for a missing async capacity | L13 |
| a state machine event is bound in one mode only, and asynchronously by at most one pair | `bindingMode` | SM-B04, SM-B05 |
| a transition port has exactly one role: caller in fusions, callee in fusions, or member of **exactly one** async pair; never fused and asynchronously bound, never in two async pairs | reported by lowering | L7 (D-16; SM-B05 for Petri net ports too) |

- **Self-relationships are legal model data** (SR RS6, NT8: a runtime unit calling its own endpoint). Only a binding pair whose two ports are in one element is a problem (`bindingSelf`).
- **Bindings on relationships between non-siblings** never contribute to any combined scope (NS §6.6: a relationship contributes iff both its elements are children of the scope). They get the warning `bindingOutsideScope`, not an error. Any port that relies on such a binding fails L4 in its scope anyway, so no smaller net can result.
- **Interaction is independent of the pairs** (SR §2.5). A request and its reply over two async pairs is still `interaction = "synchronous"` (E5 case 3).
  - When `interaction = "asynchronous"` and a sync pair has its `from` (caller) port in the drawn source, the warning `interactionVsBinding` says "the bindings make the source wait for the target". A reply pair, whose caller is in the target, never triggers it.
  - The stated value stays the one the scan uses.
- A relationship without any pair is allowed. It serves the scan, and in a combined scope it is listed as an assumption (NS §6.6, D-30).
- When a relationship has an async pair, its generated place has the relationship's ID as its qualified ID (NS §2.2).

**Why relationships live in the source element's file.** Alternatives considered:

| Option | For | Against | Verdict |
|---|---|---|---|
| **Source element's file** | One file shows what an element does and whom it calls. The pairs sit next to the source's net. No extra files. Deleting an element deletes its outgoing relationships with the file. `stack.md` §4.4 recommends it, and the UX copy (I-BM-DIRTY) assumes it. | A binding edit and a hand edit of the net share a file; per-entity conflict detection (§9.3) removes that cost. Incoming relationships are spread over other files; the model and the UI show them. | **Chosen** |
| One file per relationship | Best merge isolation | About 4,000 extra files at 2,000 elements; the pairs sit far from both nets | Rejected |
| The parent's (view owner's) file | All relationships of a view in one place | One hot file per view, so constant merge conflicts; relationships between non-siblings have no single owner | Rejected |
| Both ends | Local reading from both sides | Duplicated data that can disagree | Rejected |

The pairs are part of the relationship entity, not entities of their own. The glossary says the generated place "belongs to the relationship", and ADR 0002 says "a relationship … binds their ports".

### 6.8 Properties and final predicates [S0 built-ins; S1 final predicates and T5 `pool-share`; S4 user properties and the other templates]

Properties are stored exactly as PL §9 specifies; this section only adds what storage needs.

```toml
# elements/job.toml (excerpt): job has its own behavior model; fe1 and users are among its children
[[property]]
id = "shard-map-completes"            # required; ID alphabet (§4.1); unique in the project (E408)
label = "Shard map can always be completed"   # optional; never the headline (PL §5)
behavior = "combined"                 # required: "own" | "combined"; no default (E403)
kind = "always_reachable"             # exactly one of kind and template (E410)
predicate = "fe1/map_entries = 5"     # for never and always_reachable only (E401, E402)

[final]
combined = "users/signed_in = 4"      # optional: final predicate of the combined behavior of the children
own = "job/done = 1"                  # optional: final predicate of this element's own behavior
```

**Property entries.**

- `kind` is one of `deadlock_free`, `no_capacity_exceeded`, `no_dead_transitions`, `never` and `always_reachable` (E406). Each layer keeps its own spelling of a kind (D-33): prose "no capacity exceeded", TOML `no_capacity_exceeded`, canonical line `capacity-never-exceeded`; PL §2 maps them.
- Built-ins are **explicit entries**, so what is checked is exactly what is in the files (PL §9.2, J6, D-20). New project seeds two of them (§2.4).
- Template properties are stored by **kind, version and parameters** and expanded at lowering into exactly one property (D-11, ADR 0019): `template`, `template_version` and the parameters of PL §8.2 (`place`, `terms`, `at_most`, `pool`, `held_by`, `holder`, `k`, `target`, `preset`). Expanded text is never stored, so presets such as "its initial marking" follow edits. A held-back template (T6) is not a known template and is E411.
- Writer key order (PL §9.2): `id, label, behavior, kind, predicate, template, template_version`, then the template parameters alphabetically. Entries are sorted by `id` in the file.
- Unknown keys in an entry are kept verbatim and reported as E407, and the entry is not checked (§3.3). The key `final` inside an entry stays reserved and is E407: no property overrides the scope's final predicate in v1 (D-41, ADR 0018).
- The field rules of PL §10.5 need no parser and apply from slice 0, except E404 and E405: an own scope whose element has no behavior model, or a combined scope with no child that has one, is the key state `nothingToCheck`, not a problem (D-13, NS §6.6). The seeded built-ins of a new project are therefore never reported.

**Predicates** (in properties, in `[final]` and in template parameters) are stored verbatim as the user typed them; the app never reformats them, and the CheckKey hashes them as parsed, never rewritten (D-10).

- Their grammar is NS §5.3, as transcribed in PL §3.2. This spec defines no grammar of its own.
- **Which slice parses them (D-25, ADR 0018):**

  | Slice | `[final]` | `never`, `always_reachable`, `no_dead_transitions`, templates |
  |---|---|---|
  | S0 | Stored and written back verbatim, never parsed. A `[final]` table that holds any key is the warning `finalNotChecked` ("done when is not checked until slice 1"); deadlock-free lowers with `final none`. | Stored verbatim, never parsed. The property's key state is Unknown `PropertyNotCheckable` ["user properties are checked from slice 4"] (NS V57). Only `deadlock_free` and `no_capacity_exceeded` are checked. |
  | S1–S3 | `model::property` parses and resolves it (E1xx, E2xx, L9) and lowering copies it into deadlock-free. | as S0, except `pool-share`: its parameters are parsed and resolved (E2xx, L9, E412), it is expanded at lowering with its preconditions E304, E305 and E308, and checked (PL §8.3, D-60) |
  | S4 | as S1 | parsed, resolved and checked; E1xx, E2xx, E3xx reported |

  A `pool-share` entry in slice 0 is Unknown `PropertyNotCheckable` with the line "never holds the whole pool is checked from slice 1" instead (PL S-06).

  `no_dead_transitions` is in the right-hand column because NS §8.5 is a slice 4 property (NS §15).
- **Which places a predicate may name** (NS L9, ADR 0015, D-01):
  - Own behavior: any place or state of the element.
  - Combined behavior: place ports of the children (every state is one) and generated places of the scope (PL §4.3 rule 5, E208). A preset that lowering computes from the model ("its initial marking") stores no IDs and may cover every place.

**Final predicates** (`[final]`; UI label "done when"; D-28).

- They are model data of the scope element, one per behavior: `combined` for the combined behavior of its children, `own` for its own behavior (ADR 0003, ADR 0018). `[final]` accepts only these two keys; any other key is `schema` at the element level (§3.3). The writer order is `combined, own`.
- Lowering copies a final predicate into the deadlock-free property of that scope and behavior (`NetProperty::DeadlockFree { final }`, D-04). It is not part of the net, so editing it changes only the deadlock-free keys of that behavior and, from slice 4, the key of a `get-back-to` template with `preset = "done"`, whose expansion is the final predicate (NS §11, ADR 0022, D-48); only those properties are affected by its problems (D-47).
- From slice 1, a final predicate with an E1xx or E2xx problem (E208 included) ends exactly those properties as Unknown `PropertyNotCheckable`, each detail line prefixed "done when: " (PL §7, §10.1; ADR 0017, D-47); it is never lowered as `final none`. The net of the scope still lowers, and every other property of the scope is checked as usual. Before slice 1 it is never parsed (table above).
- Slice 1 brings the minimal form in the editor (a conjunction of `place = n`, and for a state machine with several final states the sum atom `Σ E/f = 1`); a hand-written `[final]` may be any predicate of NS §5.3 (D-05). The stored shape is the same in every slice.
- A state machine's `final` list is not a `[final]` value: the own-scope default derived from it (D-42, §6.5) is never written.

## 7. Layout files `layouts/<id>.toml`

One layout file per element (ADR 0024). It holds everything about where **this** element and its inside are drawn:

```toml
# layouts/fe1.toml
format = 1

[position]                   # where fe1 is drawn in its parent's view; the landscape when it has no parent
x = 320                      # required; integer canvas units, top-left corner
y = 80
w = 200                      # optional; default size by kind when absent
h = 120

[relationships.fe1-audit]                    # bend points of a relationship whose source is fe1 (as in §6.7)
points = [[440, 40], [560, 40]]              # optional; absent means routed automatically

[behavior.places.peer_threads]               # [S1] layout of fe1's own behavior model
x = 40
y = 0
```

- **Which file.**
  - An element's position is `[position]` in its own file, in the coordinates of its parent's view. A top-level element's `[position]` is its place in the landscape. The landscape has no file of its own; it is drawn from the layout files of the elements without a parent.
  - A relationship's bend points are `[relationships.<id>]` in the layout file of its **source**, the element whose file holds the relationship (§6.7). They are drawn in the view of its ends' common parent; v1 lays out only relationships between siblings.
  - An element's behavior-model layout (`BehaviorLayout`, design §7) is in the same file:
    - Petri net: `[behavior.places.*]` and `[behavior.transitions.*]` with `x` and `y`; places may also have `w` and `h`.
    - State machine: `[behavior.states.*]` with `x`, `y`, `w` and `h`, and `[behavior.transitions.*]` with `points`.
  - A view is drawn from the layout files of its children. No file is shared by two elements, so moving, resizing or placing two different elements touches two different files.
- **Which gestures write it.**
  - A layout-only Save (move, resize, bend points, behavior-layout moves) touches only `layouts/`, and a model edit never touches a layout file (spike E2).
  - Creating an element writes its first position into its own, new layout file, next to its element file.
  - Deleting an element deletes `elements/<id>.toml` and `layouts/<id>.toml`, and removes the bend points of the relationships that pointed to it from their sources' layout files.
  - Re-parenting and reconnecting remove entries as §4.4 says. No entry ever moves between two layout files.
- **Missing entry** means unplaced. ELK places it, and only unplaced elements are placed that way (UX brief). The position is written at the next Save. A missing file means the element and its inside are unplaced.
- **Stale entry** (`orphanLayout`, info): a layout file whose stem names no element; a `[relationships.<id>]` entry that is not an outgoing relationship of this element; a `[behavior.*]` entry whose name is not in the element's behavior model. It is kept verbatim and never written by the app.
  - A merge that brings the element back later finds its position again.
  - The app removes an entry only as part of a gesture: delete, re-parent or reconnect.
- Layout files hold only positions, sizes and bend points. A layout change never changes a CheckKey (ADR 0005, 0013) or the scan input hash (SR §3.9).
- Entries are multi-line tables rather than one-line inline tables, because git conflicts on changes to adjacent lines (§15 S2). A blank line and a header between two entries keep edits of different entries mergeable.
- The app deletes a layout file only when its element is deleted. Otherwise a file with no entries stays, holding `format = 1` and any comments.
- `[position]` assumes that v1 draws each element in exactly one view. If a later version shows one element in several views, it becomes `[position.<view>]`, which is a format bump (ADR 0024).

## 8. Suppressions file `suppressions.toml` [S5]

The shape is SR §2.7 and §3.8.

```toml
format = 1

[[suppression]]
key = "REL-NO-TIMEOUT|relationship=flush-records"   # required: the finding key of SR §3.7
disposition = "accepted"     # required: "accepted" | "not-real"
reason = "On purpose: this design reproduces the incident, in which the webservers blocked on the Kinesis put."   # required: 1–500 characters after trimming
date = 2026-11-02            # required: TOML local date, from the Clock port
```

- **Key** = the canonical finding key of SR §3.7 (`RULE-ID|field=value|…`, ID lists sorted).
  - It needs no escaping, because the ID alphabet excludes `|`, `=` and `,`.
  - Keys contain IDs only, never names, so a suppression survives display renames and layout moves.
  - New information changes the key and shows the finding again (SR §3.7): a new cycle member, a new accessor, a changed defect.
- Entries are sorted by `key`. Entity identity is the `key` (§9.1).
- **Validation** (SR §2.7). Each of these is the model problem `badSuppression`:
  - a reason that is empty or longer than 500 characters after trimming;
  - an unknown rule ID;
  - a malformed key;
  - a key used twice;
  - a `disposition` outside the two values.

  An entry with a problem suppresses nothing (the finding shows) and is kept verbatim.
- A suppression whose key matches no current finding is **not** a load problem. The scan panel lists it under "Suppressions without a finding" at display time (SR §3.8). Revision 1's `orphanSuppression` is withdrawn.
  - The IDs in its key still count as taken (§4.3), so a new element can never inherit an old suppression.
- Suppressions are in one project file rather than in element files, because a finding (a cycle, a shared database) spans several elements and has no natural owner. ADR 0020 adds it to ADR 0001's file list (D-18).

## 9. Entities: the unit of Save, journal and conflicts

### 9.1 Entity keys

| Entity key | File | Path in that file |
|---|---|---|
| `project` | `arch.toml` | the whole file except `format` |
| `element:<id>` | `elements/<id>.toml` | the top-level keys except `format`. The file exists exactly when the element exists. |
| `behavior:<id>` | `elements/<id>.toml` | `petri_net` or `state_machine` |
| `relationship:<rid>` | `elements/<source>.toml` | `relationships.<rid>`, including its `binding` array |
| `property:<pid>` | `elements/<scope element>.toml` | the `[[property]]` entry whose `id = "<pid>"` |
| `final:<id>` | `elements/<id>.toml` | `final` |
| `layout:<id>` | `layouts/<id>.toml` | `position` |
| `layout:<rid>` | `layouts/<source>.toml` | `relationships.<rid>`, the bend points; elements and relationships share one namespace (§4.2), so `layout:<id>` never collides |
| `behavior_layout:<id>` [S1] | `layouts/<id>.toml` | `behavior` |
| `suppression:<key>` | `suppressions.toml` | the `[[suppression]]` entry whose `key` equals `<key>` |

- Two entity values are **equal** when their parsed values are equal; comments, whitespace and key order do not count.
- **Array entries are identified by their `id` or `key` field, never by position.** This holds in Save, in journal images and in per-entity reload.
  - Entries that share an `id` or `key` (E408, `badSuppression`) are not entities. The app never adopts, writes or images them, and keeps them verbatim until the user fixes the file.
- An `Unloaded` behavior is an entity whose value is its raw section. The app never changes it, so it is never dirty.
- `model::EntityChange` (design §3) uses these keys. The journal (§13) stores entity values as partial files rooted at these paths.

### 9.2 Save file set

- A file is written when at least one of its entities differs between the saved snapshot and the current snapshot. Entities are compared with `Arc::ptr_eq` first, then by value.
- An element file and its layout file are deleted when their element is deleted.
- A file whose rendered bytes equal its bytes on disk is not written.
- Every write carries the hash the app last saw on disk (design §4). A file whose disk hash differs is skipped and reported as a conflict.
- A file that did not load (§3.3) is never written or deleted.

### 9.3 Reload and disk changes: conflicts per entity

When a project file F changed on disk (its hash equals neither `last_seen[F]` nor a pending write of ours):

1. Parse F. If it does not load, report it, keep the in-memory version of F's entities, and change nothing else (UX "files not loaded", V-BANNER-FNL).
2. For every entity E that maps to F in the saved snapshot, on disk or in the current snapshot, compare *base* (the saved value), *disk* and *mine* (the current value):

| base = disk | mine = base | disk = mine | Outcome for E |
|---|---|---|---|
| yes | — | — | Nothing changed on disk. Keep mine, dirty or clean. |
| no | yes | — | **Adopt disk** (`Command::AdoptFromDisk`; one undo step together with the other adoptions from the same reload). |
| no | no | yes | Both sides made the same change: adopt it, and E becomes clean. |
| no | no | no | **Conflict on E** only. The dialog groups conflicts by file and offers "keep mine" or "use disk" per entity. |

3. Afterwards:
   - the kept document for F is the disk's document;
   - `last_seen[F]` is the disk hash, also when "keep mine" was chosen (contract R10);
   - `saved` holds the disk value of every adopted entity;
   - entities kept as "mine" stay dirty, and the next Save renders them into the new document, which keeps the comments that came from disk.

So a hand-edited net (`behavior:billing`) and a binding made in the app (`relationship:billing-calls-shipping`) in the same file never conflict. A comment-only edit on disk is adopted silently, because no entity changed.

This refines design §4. It said "a dirty entity whose file changed is a conflict"; it now reads "a dirty entity whose own value changed on disk is a conflict" (design §3/§4 revision, D-37).

## 10. Loading, validation and problems

### 10.1 Load steps

1. If `arch.toml` is missing, the result is "not a project" (P-OPEN-ERR-NOTPROJECT). If it is over its byte limit, does not parse, or fails schema checks, the project does not open ("arch.toml has an error (line n): <cause>").
2. List the project files (§2.1). If `elements/` holds more than `ELEMENTS_MAX` files, the project does not open. Otherwise read each file's bytes after its byte-limit check and compute `b3:<64 lowercase hex>`, the BLAKE3 of the raw bytes. These hashes are `last_seen`.
3. For each file: strip the BOM, detect the line endings (§11.3), and parse with `toml_edit::Document::parse`. Record line and column numbers for this file's problems **now**, because `Document::into_mut()` drops spans. Then keep the `DocumentMut`.
4. Check `format` (§12.2), and migrate in memory if it is older.
5. Check the count limits (§3.5) on the document, then decode strictly, section by section (§3.3).
6. Build the `Model` from what loaded. Cross-file checks follow: duplicate IDs, references (§10.4), parent cycles, the project-wide counts.
7. `model::validate` adds the model problems (§10.3). Problems are a derived function of the Model (design §3), not stored state, so they are computed the same way after open, edit, undo and reload.

The loader never fails as a whole because of one element file (ADR 0001, FMT-2). Only three things stop a project from opening: `arch.toml`, the project-wide limits and newer formats.

### 10.2 Problem shape

`Problem { kind, code?, severity: error | warning | info, file?, line?, column?, element?, relationship?, port?: QualifiedId, property?, message }`

- This is UX contract R2's `ProblemDto` plus `code` (the NS L-rule, SM code or PL code), `severity`, `column` and `property`. SM §8.1 and PL §10 already ask for the typed cause.
- Problems found in the in-memory model (an edit in the app) carry no line number.
- A scope's `ModelIncomplete` detail line and the Problems entry are two renderings of one cause (SM §8.1). The detail line uses qualified IDs and the NS wording; the Problems entry uses display names.

### 10.3 Problem catalog

The "Blocks" column says what the problem stops:

- **file**: the file does not load.
- **behavior**: the behavior is `Unloaded`, and the scopes that contain the element fail L1.
- **scopes**: lowering fails with `ModelIncomplete` for every scope the problem touches (§10.5).
- **property**: that property is not checked.
- **scan**: the scan treats the value as not stated, or excludes the element.
- **nothing**.

| Kind | Code | Severity | When | Blocks |
|---|---|---|---|---|
| `parse` | — | error | TOML syntax error, including git conflict markers (§15 S16) | file |
| `schema` | — | error | element level: missing `format` or `kind`, unknown key, wrong type, unknown `kind`, invalid ID or name, string too long, both behavior notations; also a bad file name | file |
| `tooLarge` | LIM-2 | error | a byte or count limit of §3.5 at the file level | file (project-wide limits: the project does not open) |
| `newerFormat` | FMT-1 | fatal | `format` above what the app supports | the project does not open |
| `behaviorNotLoaded` | SM-E10, SM-E12 | error | a schema or limit failure inside `[petri_net]` or `[state_machine]` | behavior (L1: "`<element>` has a behavior model that did not load") |
| `noBehavior` | L1 | error | a child of a combined scope has no behavior model. Listed only when another child has one; with none at all, or for an own scope whose element has none, the key state is `nothingToCheck` (NS §6.6, D-13, flows R9) and nothing is reported. | scopes |
| `noWhenFull` | L2 | error | a place without `when_full` | scopes |
| `dangling` | L3 | error | `parent`, relationship `to` or an arc place names nothing | scopes. For `parent`: the element is shown in the landscape; when the parent's file did not load, §10.5 blocks that parent's combined scope. A predicate place that names nothing is a `property` problem (PL E2xx). |
| `unknownPort` | L3 | error | a binding port that does not exist | scopes |
| `unbound` | L4, SM-B07 | error | a transition port, or an input or output event, of a child is bound by no contributing relationship | scopes |
| `notAPort` | L5, SM-B02 | error | a binding names a place, a place port, a state, an internal event, or a transition without `port = true` | scopes |
| `bindingSelf` | L6, SM-B06 | error | both ports of a pair are in one element | scopes |
| `bindingAsync` | L8 | error | more than one async pair on one relationship | scopes |
| `nameClash` | L10 | error | a place and a transition of one element with the same name (NS §2.2). The file loads; the editor rejects the gesture (D-26). A state/event/transition clash is `smStructure` (SM-E04). | scopes |
| `noCapacity` | L10 (place), L13 (async pair) | error | a place or an async pair without `capacity` | scopes |
| `badInitial` | L10 | error | `initial` above `capacity` (NS V29: "x/p: initial 5 exceeds capacity 3") | scopes |
| `bindingDirection` | L11, SM-B01 | error | `from` and `to` are not ports of the relationship's two elements, one in each (either order, D-07); or an output event used as `to`, or an input event used as `from` | scopes |
| `bindingDuplicate` | L12 | error | two sync pairs on one relationship with the same (bare `from` name, bare `to` name), whatever their direction; for state machine events the full L12 check on fused transition IDs is reported under this code by `engine::lower` (SM V-25, D-50) | scopes |
| `badBinding` | L13 | error | `capacity` or `when_full` on a sync pair | scopes |
| `smStructure` | L14 (SM-E01–E09, SM-E11) | error | SM §8.3; SM-E11 only in combined scopes | scopes |
| `bindingMode` | L15 (SM-B04, SM-B05) | error | SM §8.3 | scopes |
| `duplicateId` | L3 | error | a relationship ID used in two files, or equal to an element ID | scopes: none of the clashing relationships enters the model, and every scope that could contain any of them is blocked (a missing relationship could remove a fused alternative) |
| `parentCycle` | L3 | error | the parent chain loops | scopes: the own and combined scopes of every element on the cycle |
| `property` | PL E1xx, E2xx, E4xx | error | PL §10 (E208 is NS L9). E4xx and E114 from slice 0; E1xx and E2xx from slice 1 for `[final]` and from slice 4 for properties and template parameters (D-25, D-49, §6.8) | property (Unknown `PropertyNotCheckable`, never `ModelIncomplete`; D-47); a final predicate with such a code blocks only the deadlock-free and, from slice 4, T7 "done" properties of its scope and behavior (§6.8, D-04, D-48) |
| `finalNotChecked` | D-25 | warning | slice 0 only: a `[final]` table holds a value; it is kept and written back verbatim, and deadlock-free lowers with `final none` | nothing |
| `kindLevel` | SR §3.10 | warning | the kind does not match its level, or the parent has the wrong kind | scan (the element and everything below it are excluded) |
| `badAttribute` | SR §3.10 | warning | §6.3 | scan (treated as not stated) |
| `interactionVsBinding` | SR §2.5 | warning | `interaction = "asynchronous"` with a sync pair whose `from` is in the source | nothing |
| `bindingOutsideScope` | NS §6.6 | warning | binding pairs on a relationship whose ends are not siblings | nothing |
| `smWarning` | SM-W01–W03 | warning | SM §8.3 | nothing |
| `badSuppression` | SR §2.7 | warning | §8 | that suppression suppresses nothing |
| `orphanLayout` | — | info | §7 | nothing; the entry is kept |

Proposed additions to R2's kinds:

- `schema`, `tooLarge`, `newerFormat`, `behaviorNotLoaded`
- `notAPort`, `bindingSelf`, `bindingAsync`, `bindingDirection`, `bindingDuplicate`, `badBinding`, `bindingMode`
- `nameClash`, `noCapacity`, `badInitial`, `smStructure`, `duplicateId`, `parentCycle`, `property`
- `kindLevel`, `badAttribute`, `interactionVsBinding`, `bindingOutsideScope`, `smWarning`, `badSuppression`, `orphanLayout`, `finalNotChecked`

R2 already has `noBehavior`, `unbound`, `dangling`, `unknownPort`, `noWhenFull` and `parse`.

Some SM §8.3 rows are also editor invariants (SM-B01, SM-B02, SM-B04–B06); `model::apply` Rejects them. The catalog covers them in hand-written files.

### 10.4 Every reference in the format

| From | To | If missing |
|---|---|---|
| `parent` | element | `dangling` |
| relationship `to` | element | `dangling` (drawn with an open, dashed end, `danglingEnd`) |
| binding `from` / `to` | port of an element of the relationship | `unknownPort`, `notAPort`, `bindingDirection` |
| `pre` / `post` keys | place in the same Petri net | `dangling` |
| state machine `initial`, `final`, `from`, `to`, `event` | state or event in the same state machine | `smStructure` (SM-E03, SM-E09, SM-E06, SM-E07) |
| predicate place, template parameter place | place in the scope (PL §4.3) | `property` (E201–E208) |
| layout file stem | an element | `orphanLayout` (info) |
| layout `[relationships.<rid>]` | a relationship whose source is this file's element | `orphanLayout` (info) |
| layout `[behavior.*]` key | a name in this element's behavior model | `orphanLayout` (info) |
| suppression key IDs | not checked at load; matched against findings at display time (SR §3.8) | — |

### 10.5 Effect on checks: never check a smaller net

**Problems that block scopes.** A problem whose "Blocks" column says **scopes** touches:

- for an element: its own scope and its parent's combined scope;
- for a relationship: the combined scope of its ends' common parent;
- for a property: that property only (PL §10.1); for a final predicate: the deadlock-free properties of its scope and behavior only (D-04, §6.8).

Lowering fails with `ModelIncomplete` for every touched scope (design §6: "lowering is total or fails"; NS §6.6). The problems are listed as the causes, so the UX can select them.

**An element file that does not load** contributes no entities. Its element is missing, so a combined scope could be lowered without that child. That could be a smaller net, and possibly a wrong Proved. Therefore:

- **Valid TOML that fails at the element level.**
  - The loader still reads `parent` and the relationship keys leniently, and blocks the combined scope of that parent ("elements/x.toml did not load").
  - The file's own properties and final predicates did not load either, so none of them is checked; the element is listed under "files not loaded" (V-BANNER-FNL), never shown as `nothingToCheck`.
  - Its children get `dangling` with the message "parent x did not load".
- **Not valid TOML.** The parent cannot be known, so **every combined scope in the project** is blocked until the file loads.
  - Own scopes of other elements stay checkable. An own scope consists of one element's behavior, properties and final predicate, all in that element's own file.

**Other cases:**

- **An `Unloaded` behavior** blocks, through L1, the element's own scope and its parent's combined scope, and nothing else.
- **On Reload or a disk change**, a file that loaded before keeps its last loaded version in memory. Checks run on that version, which is exactly what Save would write. The UX shows "Verdicts are for the last version that loaded" (V-BANNER-FNL).
- A broken layout file only drops one element's positions and bend points, and a broken suppressions file shows findings unsuppressed. Neither touches a scope.

## 11. Writing: canonical writer and comment preservation

### 11.1 Canonical form (new files, new entities, new keys)

1. UTF-8 without a BOM, and LF line endings in new files. **The text ends with a newline followed by one empty line** (`…\n\n`; `…\r\n\r\n` in a CRLF file, §11.3), so a table appended at the end never sits next to an edited last line (ADR 0024, §15 S21). No trailing whitespace on any line; no tabs.
2. Key order:
   - element file: `format, kind, name, parent, technology, description, replicas, zones, service`, then the tables in this order: `petri_net` or `state_machine`, `relationships.*`, `[[property]]`, `final`;
   - Petri net: all `places.*` (`name, capacity, when_full, initial, port`), then all `transitions.*` (`name, port, pre, post`);
   - state machine: `initial, final`, then `states.*` (`name`), `events.*` (`name, direction`), `transitions.*` (`name, from, event, to`);
   - relationship: `to, meaning, technology, description, interaction, timeout`, then its `[[…binding]]` pairs (`mode, from, to, capacity, when_full`);
   - property: as in PL §9.2 (`id, label, behavior, kind, predicate, template, template_version`, then the parameters alphabetically);
   - `final`: `combined, own`;
   - layout file: `format`, then `position` (`x, y, w, h`), `relationships.*` (`points`), `behavior.*`;
   - suppression: `key, disposition, reason, date`;
   - `arch.toml`: `format`, `budget` (`time_ms, max_markings`).
3. Sorting:
   - keyed tables in a group (places, transitions, states, events, relationships, relationship and behavior entries in a layout file) bytewise by key;
   - `[[property]]` entries by `id`;
   - `[[suppression]]` entries by `key`;
   - binding pairs by `mode`, then `from`, then `to`.
4. One blank line goes before every table header, and none between the keys of a table. Parent tables with no keys of their own are implicit (`Table::set_implicit(true)`), so there is no bare `[petri_net]` or `[relationships]` header. `[state_machine]` is explicit, because it holds `initial`.
5. Inline tables are used only for arc maps (`pre`, `post`), on one line with their keys sorted. Arrays (bend points, `final` state lists) also go on one line.
6. Strings are basic strings `"…"` with escapes. A string that contains a newline (`description`) is a multi-line basic string `"""` with a leading newline. Keys are bare when they match `[A-Za-z0-9_-]+` and quoted otherwise.
7. Integers are decimal, without underscores, a sign or leading zeros. Dates are `YYYY-MM-DD`.
8. Defaults are omitted only for:
   - `initial = 0` and `port = false`;
   - empty `pre`/`post` and an empty state machine `final`;
   - a behavior-part `name` equal to its key;
   - a not-stated attribute.

   `capacity`, `when_full` (on places **and** on async pairs), an element's `name` and a property's `behavior` are always written. They decide verdicts or headlines, and a reader should not need to know the defaults.

`render(model)` must be deterministic: the same model always gives the same bytes, whatever the insertion order. The CheckKey invariance test of ADR 0013 and the scan-hash fitness test of SR §3.9 build on this.

### 11.2 In-place editing of existing files (toml_edit 0.25)

The session keeps one `DocumentMut` per file (design §3). Save edits it in place:

1. An unchanged entity is left alone, and its bytes are written back identically. A document with no edits renders byte-for-byte equal to its input, arrays of tables included (§19).
2. **When a scalar value changes, the old value's decor is copied onto the new value**: the whitespace around it and the comment that follows on the same line. With toml_edit 0.25.17, `doc[..]["capacity"] = value(8)` drops the trailing comment; copying `Value::decor()` keeps it (§19).
3. A key added to an existing table is inserted after the closest preceding key in the canonical order, or at the start of the table when there is none. It is formatted canonically.
4. A new keyed table is inserted after the last sibling whose key sorts before it; failing that, before the first sibling; failing that, at the group's canonical position. It gets one blank line of prefix decor.
   - `Table::set_position` takes `Option<isize>` in 0.25.17, so the new table can usually take a position between its neighbors without renumbering every later table. When no gap exists, later positions are shifted by one.
   - A new `[[property]]`, `[[suppression]]` or binding pair is inserted with `ArrayOfTables::insert` at the index of its sorted position (§19).
   - If the existing siblings are not sorted (a hand-written order), the new table or entry goes after the last sibling. The app never reorders.
5. A removed entity's table or array entry is removed together with its sub-tables (`ArrayOfTables::remove` for entries). Comments in its decor and inside it go with it. This is the one documented loss of comments, and it happens only when the user deletes the thing the comment is about.
6. A changed inline table (`pre`, `post`) is rebuilt canonically, and the decor of the inline table value as a whole is kept.
7. A changed predicate string is replaced verbatim with what the user typed.
8. The writer never reorders existing keys or tables, never reformats untouched values, and never changes quoting, number styles or TOML 1.1 syntax that the user chose.
9. **The empty last line stays last** (§11.1 rule 1). In a parsed file, the empty last line is whitespace after the last item, which toml_edit is expected to keep as the document's trailing decor (`DocumentMut::trailing`) and to render after every table. A table, array entry or key appended at the end (rules 3 and 4) is then rendered before it, with its own one blank line of prefix decor, and the file ends in `…\n\n` with the same bytes as §11.1 gives. When Save writes a file whose rendered text does not end with an empty line (hand-edited, or trimmed by an editor), the writer appends the missing line breaks in the file's style (§11.3); that is the only change it makes outside an edited entity.
   - **Not verified.** Spike E2 ran only the canonical writer, so slice 0 must test that toml_edit 0.25 keeps the empty last line when it appends a table, an array entry and a key (§16). If it does not, the writer sets the trailing decor itself before rendering.

### 11.3 Line endings

toml_edit renders a parsed CRLF file with LF line breaks (§19). A Windows checkout with `core.autocrlf=true` would otherwise show every saved file as fully changed. The rule:

- At load, a file's style is **CRLF if its first line break is CRLF**, else LF.
- After rendering, every line break is converted to that style.
- The file hash is computed over the bytes written.
- A file with mixed line endings is written in the style of its first line break.
- Apart from the empty last line (§11.2 rule 9), nothing else is normalized.

### 11.4 Atomic writes

For each file:

1. Write `.pn-<8 random lowercase hex>.tmp` in the target's directory.
2. `sync_all`.
3. Rename it over the target.
4. Fsync the directory (Linux, macOS).

Directories (`elements/`, `layouts/`) are created on demand. A delete is a plain `remove_file` after the same hash check. ADR 0001 accepts that a set of renames is not atomic; the journal (§13) covers a torn Save.

**Windows.** `std::fs::rename` replaces an existing file, but it fails with a sharing violation while an editor, the search indexer or antivirus holds the target open.

- The writer retries the rename up to 5 times, with pauses of 20, 40, 80, 160 and 320 ms (about 0.6 s in total).
- If all retries fail, it removes the temporary file and reports a **failed write** for that file, not a conflict.
- The file's entities stay dirty, `last_seen` is unchanged, and `save_done` lists the path under `failed` (§13.3).

## 12. Versioning and migration

### 12.1 Version numbers

`format` is one integer per file. This spec is format **1**. The app supports `1 ..= CURRENT`.

- Any change to accepted keys, types, ranges, **defaults** or meaning bumps `CURRENT`, including purely additive keys. Older readers are strict (§3.3), so a clear "made by a newer version" message is better than an "unknown key" message.
- Changing a default (for example the async `when_full`) needs a migration that writes the old default explicitly.
- Design §3 keeps one `FormatVersion` per file (`Model.versions`, ADR 0020, D-37).

### 12.2 Reading

- If any project file has `format > CURRENT`, the project does not open: "<path> was saved by a newer version of this app (format <n>). Update the app to open it." (P-OPEN-ERR-NEWER). The message names the first such file in path order.
- A file with `format < CURRENT` is migrated in memory, one step at a time, by `migrate_file::<n>(doc: &mut DocumentMut, kind: FileKind) -> Result<(), MigrationError>` for n → n+1.
  - A step edits the `toml_edit` document, so comments survive, and it sets `format = n+1`.
  - A migration error is a file-level `schema` problem for that file.
- **Project steps** (`migrate_project::<n>`, for changes that move data between files) run on the in-memory project after every file has reached n.
  - Files upgrade lazily (§12.3), so some files may already be at n+1 on disk. A project step must therefore be idempotent per file.
  - Data is taken only from files that are still at n on disk.
  - When a receiving file already holds the entity with an equal value, receiving it is a no-op. With a different value, it is a `schema` problem for the receiving file ("migration conflict").
  - Format 1 has no project steps.

### 12.3 Lazy upgrade

Migrated values become the `saved` snapshot. So migrating alone makes nothing dirty, and untouched old-format files keep their bytes and their `format` number.

- A file is written in `CURRENT` only when one of its entities changes, or when a project step moved data into or out of it. In that case the project opens dirty, and the status says how many files Save will upgrade.
- **The files that one project step touched are written together.** If any of them is skipped by a conflict or a failed write, none of them is written in that Save, and they all stay dirty.
- Mixed formats in one project are legal, because every file states its own.

This keeps a format bump from rewriting about 4,000 files in one commit, and it lets a merge combine a migrated branch with an old one (§15 S14).

### 12.4 Obligations per format bump (ADR 0001, FMT-1, TEST-3)

- A golden project per version in `crates/format/tests/golden/v<n>/`. It must load forever and migrate to the same model as the current golden. Fixtures of released versions are never edited.
- A journal golden per version (§13.6).
- A test that applying a step to a file already at n+1 is rejected (the step checks `format = n`).
- The canonical writer changes only together with the version, and its insta snapshots change with it (TEST-3).

### 12.5 When format 1 is frozen

Format 1 is frozen when slice 0 is first used on a real design (dogfooding). Before that, this spec may change without a migration, because no user project exists yet. After that, every change follows §12.1–12.4.

## 13. Recovery journal [S1]

Slice 0 has no journal (design §4). The shapes below follow ADR 0011:

- after-images as TOML fragments in the project's format, each stamped with its revision;
- base file hashes and `SaveBegin`/`SaveDone` brackets;
- written only by the derive thread, in revision order.

### 13.1 Location and lock

- **Directory:** `<app local data dir>/journals/<folder name>-<first 16 hex of BLAKE3(normalized project path)>/`.
  - It is in the app data folder (ADR 0001), never in the project, so git never sees it.
  - The key is the path, because a journal is about the files at that path. Two clones get two journals.
- **Normalized project path:**
  1. `std::fs::canonicalize`.
  2. On Windows, strip the verbatim prefix (`\\?\` → nothing, `\\?\UNC\` → `\\`).
  3. On Windows and macOS, lowercase the path (both default to case-insensitive file systems).
  4. Use `/` as the separator, and hash the UTF-8 bytes.

  One project opened under two spellings therefore gets one journal.
- **App data dir:** Tauri's `app_local_data_dir()`. This is the platform's local data directory joined with the bundle identifier, unless `appDirectoriesOverride` is configured. A CLI must resolve the same directory, including any override, or it will not see the app's journals.
- **Files:**
  - `journal` (active), `journal.restoring` (during restore), `journal.new` (during rebase).
  - `aside/<UTC timestamp>-<reason>.journal`, with the reasons `mismatch`, `declined`, `crashed-restore`, `discarded`, `damaged` and `too-old`.
  - The app never deletes files in `aside/` in v1 (D-19); revisit if the folder grows past 100 files.
- **Lock:** `journal.lock` is locked with `File::try_lock` for as long as the project is open. `try_lock` is stable since Rust 1.89; the lock is exclusive, and advisory or mandatory depending on the platform.
  - `Err(TryLockError::WouldBlock)`: another window or a CLI has the project open. This session opens the project **read-only**, with the message "<name> is open in another window. This window is read-only." Two writers would corrupt the journal, and a window without a journal would break FMT-2's "every gesture is journaled" (D-19, ADR 0020). The read-only session rejects `apply`, `undo`, `redo`, `save` and `resolve_conflict` with one reason (design §5).
  - `Err(TryLockError::Error(_))`: the file system does not support locks (for example some network file systems). The session journals without a lock and logs a warning. It never reports this as "open in another window".

### 13.2 Framing

```text
petrinet-journal 1\n                        magic line, once
@record <body length in bytes> <BLAKE3 hex of body>\n
<body: one TOML document, UTF-8>\n
@record …
```

**Reading.** The reader reads frames until the first one that:

- is incomplete (a torn tail);
- has a wrong length or checksum;
- exceeds `JOURNAL_RECORD_BYTES_MAX`;
- does not parse; or
- breaks the `seq` order.

It keeps the **prefix** before that frame. A torn tail is expected after a crash and is silent. Any other stop moves a copy aside as `damaged` and is reported.

**Writing.**

- The writer **never appends to a journal it has not just written**. After open, and after every restore decision, it writes a fresh journal (§13.4), so a torn tail is never followed by valid frames.
- Each append is followed by `sync_data`.
- If an fsync fails, journaling is turned off for the session with a warning, and Save keeps working.

### 13.3 Records

Every body has `kind`, `seq` (0 for `base`, then +1 per record, no gaps) and `rev`.

| `kind` | `rev` means | Other keys |
|---|---|---|
| `base` | the snapshot revision that the files on disk represent | `format` (the `CURRENT` the journal is written in), `project` (normalized path), `created` (UTC offset date-time), `[files]`: **every** project file on disk → `b3:` hash |
| `change` | the revision this apply assigned; strictly increasing over `change` and `adopt` | `label` (optional, contract R14), `[[images]]` |
| `adopt` | the revision of the `AdoptFromDisk` or reload resolution | `[files]`: path → new expected hash or `"deleted"`; `[[images]]` (none when only "keep mine" was chosen, R10) |
| `save_begin` | the snapshot revision being saved; ≤ the highest revision journaled so far | `[writes]`: path → hash of the bytes being written, or `"deleted"` |
| `save_done` | the same as its `save_begin` | `[written]`: path → hash or `"deleted"`; `failed` (paths); `conflicts` (paths) |

**Images.** An image has `entity` (an entity key, §9.1), `file` (a relative path), and either `deleted = true` or an `[images.after]` table. That table holds the entity as a **partial file rooted at the file root**:

- a relationship's image is `[images.after.relationships.<rid>]`, with its `[[images.after.relationships.<rid>.binding]]` pairs;
- a property's image is one `[[images.after.property]]` entry;
- an element's image holds its top-level keys without `format`.

Because an image has the shape of a file, journal migration reuses the file migrations (§13.6). Only after-images are journaled; undo lives in memory (ADR 0011).

**Applying an image** replaces exactly that entity in `file`. For array entries this is the entry with the same `id` or `key`, inserted at its sorted position if absent. Applying an image also **removes the entity from any other file** where it is, so a relationship whose source was reconnected never ends up in two files.

**Deleting an element.** A `deleted` image of `element:<id>` deletes the files `elements/<id>.toml` and `layouts/<id>.toml`, and with them every entity in those files: its behavior, outgoing relationships, properties and final predicate, its position, the bend points of its outgoing relationships and its behavior layout. The writer does not image those contained entities separately. A `deleted` image of `layout:<id>` alone (a re-parent) removes only `[position]` and keeps the file. Entities of the element that live in other files are imaged explicitly: relationships that point to it, and their bend points in their sources' layout files.

### 13.4 Writer rules

- **Fresh journal** (after open, or after a restore decision): `journal.new` = magic + `base { seq 0, rev 0, files = last_seen }`. Then `sync_all`, rename to `journal`, and fsync the directory.
- **Append** (derive thread, in revision order):
  - a `change` record after every apply that changed entities, including undo and redo (the restored images are journaled under the new revision);
  - an `adopt` record after every disk adoption.
- **Save:**
  1. `save_begin` is queued under the lock, and its hashes are registered in `pending_writes` at the same time.
  2. The `save-<rev>` thread waits until every record up to the snapshot revision has been appended (design §4).
  3. After writing, `save_done` is appended.
  4. A **rebase** follows. `journal.new` = base `{ rev = snapshot rev, files = last_seen after Save }`, plus every `change` and `adopt` with `rev > snapshot rev` in the original order, renumbered from `seq 1`. Then `sync_all`, rename and directory fsync.

  Records newer than the snapshot are never dropped (ADR 0011).
- **Close:** after a clean close with nothing unsaved, the journal is deleted. After "Close without saving" or "Quit without saving", it is moved aside as `discarded`. The user threw those changes away, and offering them again would be wrong; keeping a copy costs nothing.

### 13.5 Restore algorithm

1. If `journal.restoring` exists, the app died during a restore. Offer only "Open without restoring" (UX D-RESTORE-CRASHED), and move the file aside as `crashed-restore`.
2. Read the prefix of `journal` (§13.2) and fold the expected hashes:
   - start with `E = base.files` and `P = none`;
   - for each record in order: `adopt` sets `E[path]` (`"deleted"` removes the path); `save_begin` sets `P = writes`; `save_done` applies `written` to `E` and clears `P`.
3. Compare with the project files on disk, `D`. For every path in `E ∪ D ∪ P`, the path matches if `D[path] = E[path]`, or if `P` is set and `D[path] = P[path]`. "Absent" equals absent and equals `"deleted"`.
   - One mismatch means no offer: the journal is moved aside as `mismatch` (D-RESTORE-MISMATCH).
   - The comparison covers **all** project files, because after-images are applied to the files on disk, and any file that changed since may conflict with an image.
4. N = the number of `change` records whose `rev` is above the `rev` of the last `save_done` (or of the base, when there is none).
   - If N = 0, there is nothing to offer: write a fresh journal.
   - Otherwise offer "Restore N unsaved changes" (R13) or "Open without restoring". Declining moves the journal aside as `declined`.
5. Restore:
   1. Rename `journal` to `journal.restoring` and fsync the directory.
   2. Load the disk.
   3. Apply every image in `seq` order (§13.3). The last image of an entity wins, including its `file`.
   4. `saved` = the disk model and `current` = the result, so the project is dirty.
   5. Write a fresh journal whose base is the disk, followed by one `change` record with the images of every entity that differs between `current` and `saved`.
   6. Delete `journal.restoring`. Validation runs as usual.

After-images are idempotent. A torn Save, in which some files hold new bytes and some old, therefore converges in step 5 (ADR 0011).

### 13.6 Journal and format versions

- Records are written in the journal's base `format`, which is the app's `CURRENT`. This is what ADR 0011's "the project's format version" means once files carry their own versions.
- A journal with an older `format` is migrated before replay:
  - each `after` is a partial file of a known kind (from `file`), so it is wrapped as `format = <old>` plus the partial and run through the file steps;
  - the entity is then read back at its path;
  - a project step declares how entity keys and paths map across it; identity is the default.
- A journal that cannot be migrated is moved aside as `too-old`, with the message "These unsaved changes were made with an older version of the app and cannot be restored automatically. They are kept in <path>."
- A journal with a newer `format` than the app is never replayed.

### 13.7 Example journal

The lengths and hashes after `@record` are illustrative and shortened; real hashes have 64 hex digits. The `[files]` table of the base is elided. A real base lists all 18 files of §14, and a base that omits a file on disk would always mismatch in §13.5 step 3.

```text
petrinet-journal 1
@record 412 9f3a…
```
```toml
kind = "base"
seq = 0
rev = 0
format = 1
project = "/home/nils/designs/kinesis-incident"
created = 2026-11-02T14:03:11Z

[files]
"arch.toml" = "b3:6f1c…"
"elements/fe1.toml" = "b3:0a9e…"
"layouts/users.toml" = "b3:41d2…"
# … 15 more files elided in this example
```
```text
@record 498 c07b…
```
```toml
kind = "change"
seq = 1
rev = 1
label = "Set capacity of fe1/peer_threads"

[[images]]
entity = "behavior:fe1"
file = "elements/fe1.toml"

[images.after.petri_net.places.map_entries]
name = "shard map entries"
capacity = 8
when_full = "overflow"
port = true

[images.after.petri_net.places.peer_threads]
name = "peer threads"
capacity = 8
when_full = "overflow"

[images.after.petri_net.transitions.open_peer_thread]
port = true
post = { peer_threads = 1 }

[images.after.petri_net.transitions.receive_info]
port = true
pre = { peer_threads = 1 }
post = { map_entries = 1, peer_threads = 1 }
```
```text
@record 214 5e10…
```
```toml
kind = "change"
seq = 2
rev = 2
label = "Delete Users (stand-in)"

[[images]]
entity = "element:users"
file = "elements/users.toml"
deleted = true
```
```text
@record 151 a4c2…
```
```toml
kind = "save_begin"
seq = 3
rev = 2

[writes]
"elements/fe1.toml" = "b3:77c0…"
"elements/users.toml" = "deleted"
"layouts/users.toml" = "deleted"
```

The `element:users` image deletes `elements/users.toml` and `layouts/users.toml`, and with them `behavior:users`, `relationship:sign-in` and `layout:users` (§13.3). Nothing else is imaged: `users` has no incoming relationship, and `sign-in` has no bend points. If the app dies after `save_begin`, restore accepts each of the three written files at either its base hash or its `writes` hash (step 3), and every other file at its base hash. N = 2.

## 14. Complete example: E5 case 2 (Kinesis), before the fix

The E5 case `docs/product/experiments/e5/case2_kinesis_bad.toml`, written in format 1. This is the golden fixture `crates/format/tests/golden/v1/kinesis-incident/`.

**Slice.** The fixture needs drill-down (two systems with one view each, slice 3), a final predicate (slice 1, D-05) and one user property (slice 4). A slice 0 user, who has one view, would draw the six containers side by side as one combined scope and check both halves as one union net (NS §13.2). The fixture as written has two scope elements, and lowering reads `parent`, so every slice lowers it as two scopes. The slice 0 reader loads and round-trips every file byte for byte (§16), stores the `[final]` and `predicate` strings without parsing them, and reports the per-slice results listed under "Expected load report".

**Mapping from E5:**

- E5's two property scopes become two software systems, because a combined scope is the children of one element (ADR 0003, NS §6.6).
- Every place keeps E5's implicit `overflow`, so ADR 0014 gives E5's verdicts.
- `users` is a **container stand-in**, not a person. A child of a combined scope is an element whose parent is the scope element, and a Person has no parent (D-13, SR §2.1), so only a container can carry the sign-in behavior inside `cognito`.
- `kinesis-api` is a stand-in container for the same reason: Kinesis as Cognito sees it must be a child of `cognito`.
- `fe1/map_entries` and `users/signed_in` are **place ports**: a combined-scope predicate or final predicate names only place ports and generated places (NS L9, ADR 0015; NS §13.2 does the same).
- IDs use hyphens where natural (`fe-peers`, `membership-updates`); NS §13.2 writes the same design with underscores and `r1`–`r4`. Both are valid IDs (§4.1); only the bytewise transition order differs, and §19 re-ran the explorer on these IDs.
- The built-in properties are explicit entries (PL §9.2).
- The comments in `arch.toml`, `fe1.toml` and `cognito-web.toml` are user comments, which the writer keeps (§11.2).
- Every placed element has its own layout file (§7, ADR 0024). `cognito` and `kinesis-data-streams` have no parent, so their `[position]` is in the landscape.
- Each block below shows a file without its empty last line: the file on disk is the block's text followed by one more line break (§11.1 rule 1).

```text
kinesis-incident/
  arch.toml
  elements/cognito.toml  cognito-web.toml  fe-peers.toml  fe1.toml
           kinesis-api.toml  kinesis-data-streams.toml  membership.toml  users.toml
  layouts/cognito.toml  cognito-web.toml  fe-peers.toml  fe1.toml
          kinesis-api.toml  kinesis-data-streams.toml  membership.toml  users.toml
  suppressions.toml
```

`arch.toml`
```toml
# E5 case 2 (before the fix): AWS Kinesis Data Streams and Cognito, us-east-1, 2020-11-25.
# Source: https://aws.amazon.com/message/11201/
format = 1
```

`elements/kinesis-data-streams.toml`
```toml
format = 1
kind = "software_system"
name = "Kinesis Data Streams"
description = "Front-end fleet after the scale-up. Each front-end server opens one OS thread per peer."

[[property]]
id = "kinesis-deadlock-free"
behavior = "combined"
kind = "deadlock_free"

[[property]]
id = "kinesis-no-capacity-exceeded"
behavior = "combined"
kind = "no_capacity_exceeded"

[[property]]
id = "shard-map-completes"
label = "Shard map can always be completed"
behavior = "combined"
kind = "always_reachable"
predicate = "fe1/map_entries = 5"
```

`elements/membership.toml`
```toml
format = 1
kind = "container"
name = "Membership service"
parent = "kinesis-data-streams"
description = "Publishes the peers of fe1. The fleet grew to 6 servers, so fe1 has 5 peers."

[petri_net.places.listed]
capacity = 8
when_full = "overflow"

[petri_net.places.members]
capacity = 8
when_full = "overflow"
initial = 5

[petri_net.transitions.publish]
port = true
pre = { members = 1 }
post = { listed = 1 }

[relationships.membership-updates]
to = "fe1"
meaning = "publishes peers to"
interaction = "asynchronous"

[[relationships.membership-updates.binding]]
mode = "async"
from = "membership/publish"
to = "fe1/open_peer_thread"
capacity = 8
when_full = "overflow"
```

`elements/fe-peers.toml`
```toml
format = 1
kind = "container"
name = "Other front-end servers"
parent = "kinesis-data-streams"
description = "The 5 peers of fe1, folded into one element."

[petri_net.places.sent]
capacity = 8
when_full = "overflow"

[petri_net.places.up]
capacity = 8
when_full = "overflow"
initial = 5

[petri_net.transitions.send_info]
port = true
pre = { up = 1 }
post = { sent = 1 }

[relationships.shard-info]
to = "fe1"
meaning = "sends shard info to"
interaction = "asynchronous"

[[relationships.shard-info.binding]]
mode = "async"
from = "fe-peers/send_info"
to = "fe1/receive_info"
capacity = 8
when_full = "overflow"
```

`elements/fe1.toml`
```toml
format = 1
kind = "container"
name = "Front-end server fe1"
parent = "kinesis-data-streams"
description = "One front-end server. It opens one OS thread per peer."

[petri_net.places.map_entries]
name = "shard map entries"
capacity = 8
when_full = "overflow"
port = true

[petri_net.places.peer_threads]
name = "peer threads"
capacity = 4                 # OS thread limit, scaled down
when_full = "overflow"       # going past the limit is the incident

[petri_net.transitions.open_peer_thread]
port = true
post = { peer_threads = 1 }

[petri_net.transitions.receive_info]
port = true
pre = { peer_threads = 1 }
post = { map_entries = 1, peer_threads = 1 }
```

`elements/cognito.toml`
```toml
format = 1
kind = "software_system"
name = "Cognito"
description = "Sign-in path. Webservers block on backlogged Kinesis buffers (latent bug)."

[[property]]
id = "cognito-deadlock-free"
behavior = "combined"
kind = "deadlock_free"

[[property]]
id = "cognito-no-capacity-exceeded"
behavior = "combined"
kind = "no_capacity_exceeded"

[final]
combined = "users/signed_in = 4"
```

`elements/users.toml`
```toml
format = 1
kind = "container"
name = "Users (stand-in)"
parent = "cognito"
description = "Stands in for 4 sign-ins. A container, because a person has no behavior model inside a system."

[petri_net.places.signed_in]
name = "signed in"
capacity = 4
when_full = "overflow"
port = true

[petri_net.places.want]
capacity = 4
when_full = "overflow"
initial = 4

[petri_net.transitions.login]
port = true
pre = { want = 1 }
post = { signed_in = 1 }

[relationships.sign-in]
to = "cognito-web"
meaning = "signs in through"
interaction = "synchronous"

[[relationships.sign-in.binding]]
mode = "sync"
from = "users/login"
to = "cognito-web/auth"
```

`elements/cognito-web.toml`
```toml
format = 1
kind = "container"
name = "Cognito webserver"
parent = "cognito"
description = "Buffers 2 records for Kinesis."
replicas = 3
zones = 3

[petri_net.places.buf_free]
name = "free buffer slots"
capacity = 2
when_full = "overflow"
initial = 2

[petri_net.places.buf_used]
name = "used buffer slots"
capacity = 2
when_full = "overflow"

[petri_net.transitions.auth]
port = true
pre = { buf_free = 1 }       # waits for a free slot: the blocking put
post = { buf_used = 1 }

[petri_net.transitions.flush]
port = true
pre = { buf_used = 1 }
post = { buf_free = 1 }

[relationships.flush-records]
to = "kinesis-api"
meaning = "puts records to"
interaction = "synchronous"
timeout = "no"

[[relationships.flush-records.binding]]
mode = "sync"
from = "cognito-web/flush"
to = "kinesis-api/put"
```

`elements/kinesis-api.toml`
```toml
format = 1
kind = "container"
name = "Kinesis API (stand-in)"
parent = "cognito"
description = "Kinesis as Cognito sees it. It can fail for good."
replicas = 3
zones = 3

[petri_net.places.down]
capacity = 1
when_full = "overflow"

[petri_net.places.up]
capacity = 1
when_full = "overflow"
initial = 1

[petri_net.transitions.fail]
pre = { up = 1 }
post = { down = 1 }

[petri_net.transitions.put]
port = true
pre = { up = 1 }
post = { up = 1 }
```

`layouts/cognito.toml`
```toml
format = 1

[position]
x = 480
y = 0
```

`layouts/cognito-web.toml`
```toml
format = 1

[position]
x = 240
y = 0
```

`layouts/fe-peers.toml`
```toml
format = 1

[position]
x = 0
y = 160
```

`layouts/fe1.toml`
```toml
format = 1

[position]
x = 320
y = 80

[behavior.places.map_entries]
x = 200
y = 0

[behavior.places.peer_threads]
x = 0
y = 0

[behavior.transitions.open_peer_thread]
x = 0
y = 120

[behavior.transitions.receive_info]
x = 200
y = 120
```

`layouts/kinesis-api.toml`
```toml
format = 1

[position]
x = 480
y = 0
```

`layouts/kinesis-data-streams.toml`
```toml
format = 1

[position]
x = 0
y = 0
```

`layouts/membership.toml`
```toml
format = 1

[position]
x = 0
y = 0

[relationships.membership-updates]
points = [[160, 40], [320, 40]]
```

`layouts/users.toml`
```toml
format = 1

[position]
x = 0
y = 0
```

`suppressions.toml`
```toml
format = 1

[[suppression]]
key = "REL-NO-TIMEOUT|relationship=flush-records"
disposition = "accepted"
reason = "On purpose: this design reproduces the incident, in which the webservers blocked on the Kinesis put."
date = 2026-11-02
```

**Expected load report.**

- Slices 1–5: no problems. Both combined scopes lower: every transition port is bound by a contributing relationship (`kinesis-api/fail` is not a port, so it fires on its own), every pair has its two ports in the relationship's two elements (L11), and the predicate and the final predicate name place ports (L9).
- Slice 0: one warning, `finalNotChecked` on `cognito` (`[final] combined` is kept and written back verbatim; `cognito-deadlock-free` lowers with `final none`). No predicate string is parsed.
- Before slice 4, `shard-map-completes` has the key state Unknown `PropertyNotCheckable` ["user properties are checked from slice 4"] (D-25, NS V57); every built-in entry is checked from slice 0.

**Expected verdicts.** These were computed with a throwaway explorer, written from NS §4, §6, §8, §10.1 and §12, on the files above (§19). They reproduce NS §13.2 and `e5/results.txt`.

- Fused IDs use bare names (NS §2.2).
- As in NS §14, "stored" is the number of stored markings at discovery.
- On a Disproved row, #R and #O are oracle facts from a full exploration, not part of the evidence.

| Property (scope, behavior) | Verdict | Evidence | stored | #R | #O |
|---|---|---|---|---|---|
| `kinesis-no-capacity-exceeded` (`kinesis-data-streams`, combined; E5 P1) | Disproved | 10 steps: `membership/publish` and `fe1/open_peer_thread`, alternating ×5; `Overflow { fe1/peer_threads, 4, 5 }`. No `NoOverflowPlace` certificate, because the scope has `overflow` places (D-03). | 176 | 330 | 21 |
| `shard-map-completes` (`kinesis-data-streams`, combined; E5 P2) [S4] | Unknown `CapacityExceededFirst { fe1/peer_threads, 10 steps }` | Bad = ∅, O ≠ ∅ | — | 330 | 21 |
| `kinesis-deadlock-free` (`kinesis-data-streams`, combined; not in E5; no final predicate) | Unknown `CapacityExceededFirst { fe1/peer_threads, 10 steps }` | no deadlock, O ≠ ∅ | — | 330 | 21 |
| `cognito-deadlock-free` (`cognito`, combined; E5 P3). The verdict is the same with F = `users/signed_in = 4` [S1] and with F = none [S0]: the deadlock marking has `users/signed_in = 2`, so it is not final. | Disproved | 3 steps: `kinesis-api/fail`, `sign-in/login+auth`, `sign-in/login+auth` → `cognito-web/buf_used = 2, kinesis-api/down = 1, users/signed_in = 2, users/want = 2`. Waiting: `flush-records/flush+put ← kinesis-api/up NeedsTokens{1,0}`; `kinesis-api/fail ← kinesis-api/up NeedsTokens{1,0}`; `sign-in/login+auth ← cognito-web/buf_free NeedsTokens{1,0}`. | 7 | 24 | 0 |
| `cognito-no-capacity-exceeded` (`cognito`, combined; not in E5) | Proved, exhaustive, 24 markings, confirmed by the second explorer | No `NoOverflowPlace` certificate, because the scope has `overflow` places (D-03). | — | 24 | 0 |

**Expected scan result.** Hand-derived from SR §4–§8, not run (there is no scan code yet):

- Finding REL-NO-TIMEOUT, severity High, on `flush-records` (synchronous, `timeout = "no"`), suppressed with disposition *accepted*.
- Not checkable REL-NO-TIMEOUT on `sign-in`: timeout not stated.
- Nothing from ARCH-CYCLE, RUN-SYNC-CYCLE or DATA-SHARED-DB. The two asynchronous relationships are not judged by REL-NO-TIMEOUT.
- One model-level Not checkable REL-SPOF entry: "No Person uses an element, so REL-SPOF has no flow to follow" (SR §8; `users` is a container stand-in).
- Footer: 1 finding, 1 suppressed, 0 not real.

**Second golden: `kinesis-incident-block/`** (an ADR 0014 round trip). It is the same project with two changes: `cognito-web` is written with one blocking buffer place, and every place of `users` and `kinesis-api` is set to `when_full = "block"`.

```toml
format = 1
kind = "container"
name = "Cognito webserver"
parent = "cognito"
description = "Buffers 2 records for Kinesis; a put into a full buffer waits."
replicas = 3
zones = 3

[petri_net.places.buf]
name = "buffered records"
capacity = 2
when_full = "block"

[petri_net.transitions.auth]
port = true
post = { buf = 1 }

[petri_net.transitions.flush]
port = true
pre = { buf = 1 }

[relationships.flush-records]
to = "kinesis-api"
meaning = "puts records to"
interaction = "synchronous"
timeout = "no"

[[relationships.flush-records.binding]]
mode = "sync"
from = "cognito-web/flush"
to = "kinesis-api/put"
```

Expected:

- `cognito-deadlock-free` (F = none [S0] and F = `users/signed_in = 4` [S1]): Disproved with the same 3 steps, stored 7, #R 24. The last marking is `cognito-web/buf = 2, kinesis-api/down = 1, users/signed_in = 2, users/want = 2`. Waiting: `flush-records/flush+put ← kinesis-api/up NeedsTokens{1,0}`; `kinesis-api/fail ← kinesis-api/down OutputFull{1,2}, kinesis-api/up NeedsTokens{1,0}`; `sign-in/login+auth ← cognito-web/buf OutputFull{2,3}` (the blocking put).
- `cognito-no-capacity-exceeded`: Proved by the certificate `NoOverflowPlace { drop_places: [] }`, markings 0, accepted by `recheck::check_certificate` on its own composition (NS §9.5, D-03). No place of the scope is `overflow`.
- The Kinesis scope is unchanged.

**The fix** (`case2_kinesis_fixed.toml`) touches five element files and no layout file:

- `initial = 3` in `membership.toml` and in `fe-peers.toml`;
- the predicate `fe1/map_entries = 3` in `kinesis-data-streams.toml`;
- in `cognito-web.toml`, a `discard` transition port plus a **second binding pair** `cognito-web/discard → kinesis-api/error` on `flush-records`;
- an `error` transition port in `kinesis-api.toml`.

The second pair shows why a relationship holds a list of pairs (§6.7).

## 15. Git three-way merge analysis

Each scenario was run with git 2.43.0 (default `ort` strategy) on the example project of §14:

1. Branch `b` and `main` each committed their edit.
2. `b` was merged into `main`.
3. The throwaway reader of §19 loaded the result.

"Clean" means git merged without a conflict. Scenarios marked "reasoned" were not run.

S1–S20 were run on the revision 3 format: one layout file per view, and files ending in a single newline. Revision 4 changes both (ADR 0024). Spike E2 (`docs/product/experiments/e2-toml-merge.md`) re-ran the layout and end-of-file cases on the new format (its variant "+ both", git 2.43.0, `myers` and `histogram` alike); rows whose outcome changed say so and name the E2 scenario. S21 is new from E2.

| # | Concurrent edits (main / b) | Git | What the app does afterwards |
|---|---|---|---|
| S1 | Move `fe1` / move `fe-peers` in the same view | clean | Loads. Layout only, so no CheckKey changes. Revision 4: two different files, `layouts/fe1.toml` and `layouts/fe-peers.toml` (E2 5a). |
| S2 | Move `fe1` in x / move `fe1` in y | **conflict** (adjacent lines) | The user picks. Layout only. Revision 4: the same, in `layouts/fe1.toml` (E2 5b). |
| S3 | Place capacity in `fe1.toml` / async pair capacity in `membership.toml` | clean | Loads; checks run on the combined design. |
| S4 | Place capacity in `fe1.toml` / new relationship in `fe1.toml` | clean | Loads. Same file, different entities, separated by headers. |
| S5a | Two new relationships from `membership` whose IDs sort to different places | clean | Loads |
| S5b | Two new relationships from `membership` that both sort to the end | **conflict** (same insertion point) | Rebuild both tables from the two sides (below); the loader validates the result. Revision 4: still a conflict (E2 3c); it stays until the merge driver (D-19). |
| S6a | Two new layout entries in one view that sort apart | clean | Loads. The scripted test added only the layout entries, so both are `orphanLayout`; with the element files added too, nothing is reported. Revision 4: two new files (E2 3a). |
| S6b | New layout entries `fe2` / `fe3`, both sorting before `membership` | **conflict** (same insertion point) | Revision 4: **clean**. Each new element brings its own layout file, so no file is shared (E2 3b). In revision 3, keeping both sides of the hunk lost a line (below). |
| S7 | Delete `fe-peers` / edit `fe-peers` | **conflict** (modify/delete) | Correct: the user decides. |
| S8 | Delete `membership` / add a relationship from `fe1` to `membership` | clean | A `dangling` relationship `to`. This script also leaves `orphanLayout` entries, which the app's delete would have removed. The touched scope fails lowering (L3) and is never checked smaller. |
| S9 | Both create an element named "Billing" | **conflict** (add/add `elements/billing.toml`) | Both picked the same slug. If both are wanted, rename one file, which changes that ID (§4.4). |
| S10 | Display rename of `fe1` / its capacity | clean | Loads. Only the capacity changes CheckKeys. |
| S11 | The same capacity changed to 8 / to 6 | **conflict** | Correct |
| S12a | Two suppressions whose keys sort apart | clean | Loads |
| S12b | Two suppressions that both sort to the end | **conflict** (same insertion point) | Rebuild both entries from the two sides (below) |
| S13a | Two `[[property]]` entries on one element whose IDs sort apart | clean | Loads |
| S13b | Two `[[property]]` entries that both sort to the end | **conflict** (same insertion point) | Rebuild both entries from the two sides (below) |
| S14 | (reasoned) One branch upgrades `fe1.toml` to format 2; the other edits `membership.toml` | clean | Mixed formats are legal (§12.3). |
| S15 | Both branches add a relationship `feed`, from different source elements | clean | `duplicateId`: neither enters the model, and every scope that could contain either is blocked until one is renamed (§10.3). |
| S16 | Conflict markers committed by mistake | — | `parse` (checked with toml_edit 0.25.17 and Python `tomllib`). The file does not load, and §10.5 applies, so there is no wrong verdict. |
| S17 | Two new binding pairs appended to the same relationship | **conflict** (same insertion point) | Rebuild both pairs from the two sides (below); the loader checks L8 and L12. |
| S18 | Edit the predicate of one property / add a property that sorts before it in the same file | clean | Loads |
| S19 | Display rename of place `fe1/peer_threads` / a new property that names `fe1/peer_threads` | clean | Loads. A display rename never changes the ID, so the new reference resolves. Revision 1's ID-changing place rename would have left it dangling. |
| S20 | Change the `to` port of the `sign-in` pair / change the `initial` of `users/want` in the same file | clean | Loads. In this script the new pair targets `cognito-web/flush`, so lowering then reports L4 for `cognito-web/auth` and L7 for `cognito-web/flush`. The semantic checks run on every merge result. |
| S21 | Edit the last line of a file / append a table after it: the last value of `fe1.toml` and a new relationship from `fe1` (E2 2e); in revision 3 also a move of the last layout entry and a new entry after it (E2 5f) | revision 3: **conflict** (adjacent lines); revision 4: clean | Revision 4: the empty last line (§11.1 rule 1) is an unchanged line between the edit and the append, as a blank line already is in the middle of a file (S4). |

**Resolving an insertion conflict.** Do not just delete the conflict markers and keep both sides of the hunk. Git shrinks a conflict hunk by moving lines that both sides share out of it, so a block can lose a line. E2 3b, on the revision 3 layout, gave:

```text
<<<<<<< HEAD
[elements.audit]
x = 0
=======
[elements.billing]
x = 320
>>>>>>> b
y = 320
```

Keeping both sides leaves `audit` without its `y`. Instead, rebuild each block whole from its own side: copy it from `git show :2:<path>` (ours) and `git show :3:<path>` (theirs), or take one side's file with `git checkout --ours <path>` and redo the other side's addition in the app. What a mistake costs depends on the file:

- **A layout file** that then does not load leaves that element's position and bend points unplaced (§3.3). ELK places them again. It is never a wrong verdict.
- **An element file** with a lost line either fails to load or reports a model problem when the line held a required key (§3.3, §10.3), and §10.5 blocks the scopes it could change. A lost line that held an optional key with a default (an async pair's `when_full`, for example) loads as the default instead, which is a different model. Review the merged file before running a check.

**Conclusions.**

- Edits to **different files always merge**.
- Edits to **different entities in one file merge** as long as at least one unchanged line separates them. Multi-line tables with a header between entities guarantee this, for keyed tables and for arrays of tables alike. One-line inline entries would not, because git treats changes on adjacent lines as a conflict (S2).
- Since revision 4, no file is shared by two elements' layouts, and an append at the end of a file is separated from the last value line by the empty last line (S6b, S21; E2: 17 of 18 disjoint edit pairs merge clean, and every clean merge was byte-identical to the writer's output for both edits).
- The remaining conflicts are:
  - **insertions at the same sorted position inside one file** (S5b, S12b, S13b, S17): two new relationships, properties, suppressions or binding pairs in one file whose IDs sort into the same gap;
  - real conflicts (S2, S7, S9, S11).

  Sorting by ID spreads insertions; appending at the end would make every concurrent addition conflict. Resolve an insertion conflict by rebuilding both blocks (above), and the loader then checks the result. No further file split is planned for these: one file per relationship was rejected in §6.7, and it would not cover properties, suppressions or binding pairs (ADR 0024).
- **Textually clean but semantically broken** merges (S8, S15, S20) are caught by the loader or by lowering, and §10.5 keeps them from producing a smaller net.
- Verdicts are never stored (ADR 0005), so a merge can never bring in a stale Proved; every check runs again on open.
- A custom git merge driver that merges per entity (`petrinet merge-file %O %A %B`) would make the insertion conflicts clean. It is tooling, not format, so it can come after the freeze. It is not in v1 (D-19, ADR 0020); revisit after slice 3 if the dogfooding journal shows insertion conflicts more than once a month.
- Recommended for user repositories (documented, not written by the app): `*.toml text eol=lf` in `.gitattributes`. Then §11.3 rarely has to restore CRLF at all.

## 16. Tests the `format` crate must have

- **Golden projects** per format version (§12.4):
  - `golden/v1/kinesis-incident/` and `golden/v1/kinesis-incident-block/`, with their expected load reports and `Model`s;
  - the NS, SM and PL fixtures that can be written as files (NS V51–V54 cannot, §3.4).
- **Round trip:** `parse(render(m)) == m`, and `render(parse(render(m))) == render(m)` byte for byte.
  - A proptest covers random models, including state machines, multiple binding pairs and template properties.
  - A no-op render of every golden file equals its bytes, in LF and in CRLF (TEST-3, FMT-3).
- **Writer snapshots** (TEST-3): insta snapshots of the canonical writer for every file kind.
- **Empty last line** (§11.1 rule 1, §11.2 rule 9, ADR 0024). Slice 0 must have this test, because E2 did not run toml_edit:
  - every file the canonical writer renders ends with `\n\n` (`\r\n\r\n` for a CRLF file);
  - with the in-place writer (toml_edit 0.25), appending a keyed table, an array-of-tables entry and a key at the end of an existing file keeps the empty line last and gives the same bytes as the canonical writer;
  - a file without the empty line gets it back when Save writes the file, and nothing else outside the edited entity changes.
- **Layout files** (§7): a move, resize, bend-point edit or behavior-layout move changes only the layout file of the element moved (for bend points, of the relationship's source); creating an element writes `elements/<id>.toml` and `layouts/<id>.toml`, and deleting it deletes both; a top-level element's `[position]` is drawn in the landscape; `layouts/_landscape.toml` is `schema` "file name is not a valid ID"; a layout file whose stem names no element, and a `[relationships.<rid>]` entry in a file that is not `<rid>`'s source, are `orphanLayout` and kept verbatim.
- **Comment survival:** add a comment, edit a value in the same table, save; the comment is still there. This covers the decor copy of §11.2 rule 2, which a naive assignment breaks.
- **Insertion position:** a new place, relationship, relationship or behavior entry in a layout file, property, suppression or binding pair lands at its sorted position. In an unsorted hand-written group, the new item goes at the end.
- **Strictness per section:** every misspelled key in a fixture gives the right problem, line and suggestion, and fails exactly the unit of §3.3:
  - element level → the file does not load;
  - behavior → `Unloaded` (SM V-19);
  - property → E407, with the entry kept on save (PL E-31).
- **Limits:** every constant of §3.5 loads at its limit, and one above gives the named problem without allocating for the excess. For example, a file of `ELEMENT_FILE_BYTES_MAX + 1` bytes never reaches the parser.
- **Blocking**, asserted through `lower` (no smaller net):
  - an element file that is not valid TOML blocks every combined scope, and no own scope of another element;
  - an element file that fails at the element level blocks its parent's combined scope;
  - an `Unloaded` behavior fails L1 for exactly its two scopes;
  - an element file with an unknown `kind` does not load, keeps its bytes on Save, and blocks its parent's combined scope (D-19).
- **IDs:**
  - a golden slug table: non-ASCII, digits, empty, `and`/`or`/`not`, device names, `__`, trailing `-`, the 40/48 cut, collisions up to `-10`;
  - the taken set includes dangling references, so PL E-32 yields `waiting-2`;
  - `elements/Billing.toml` is `schema` "file name is not a valid ID" and blocks per §10.5 (D-12).
- **Bindings (D-07, D-16):**
  - the §6.7 call-and-reply relationship loads with no problem and lowers to `r1/call+accept_a` and `r1/return_a+reply`;
  - `api/call → billing/serve` and `billing/call → api/serve` on one relationship give `bindingDuplicate`;
  - a pair whose two ports are both outside the relationship's elements gives `bindingDirection`; an input event used as `from` gives `bindingDirection` at either end;
  - a Petri net transition port in two async pairs fails L7.
- **Slices of predicate text (D-25):** in the slice 0 build, a `[final]` table gives `finalNotChecked`, a syntactically broken `predicate` or `[final]` string loads with no E1xx and is written back byte for byte, and a `kind = "never"` entry has the key state `PropertyNotCheckable`; in the slice 1 build the same `[final]` string gives its E1xx line and ends only the deadlock-free properties of its scope and behavior as Unknown `PropertyNotCheckable` (D-47).
- **Children (D-13):** a hand-written Person with `parent` is `kindLevel` and still fails L1 in that parent's combined scope when it has no behavior model and another child has one.
- **Per-entity reload** (§9.3): every row of the table, the comment-only disk edit, and array entries matched by `id` after a reorder on disk.
- **Journal:**
  - truncation at every byte offset recovers a prefix (design §8);
  - a bit flip stops at the damaged frame;
  - `seq` gaps are rejected;
  - the fold of §13.5 over `adopt`, `save_begin` and `save_done`;
  - rebase keeps records newer than the snapshot;
  - restore after a torn Save converges;
  - an image of a reconnected relationship removes it from its old file;
  - an element deletion deletes its contained entities;
  - a journal golden per format version;
  - `TryLockError::WouldBlock` gives a read-only session, and `TryLockError::Error` gives a journaled one.
- **Merge corpus** (in CI, where git is available): the S-scenarios of §15 as tests.
  - S1, S3, S4, S5a, S6a, S6b, S10, S12a, S13a, S18, S19 and S21 must merge clean, so a writer change cannot quietly make the format less mergeable.
  - The E2 scenarios, rewritten for revision 4: every disjoint pair except 3c merges clean, and the merged tree is byte-identical to the writer's output for the model with both edits; every layout gesture touches only `layouts/`.
  - S8, S15 and S20 must produce `dangling`, `duplicateId` and the L4/L7 lowering failure respectively, and block lowering, so a broken merge is never checked.
- **Oracle** (design §8): every derived value of a session after random edits equals the same function on a model loaded fresh from the rendered files.

## 17. Decisions applied, amendments and follow-ups

### 17.1 Decisions applied in revisions 3 and 4

| Decision | What it changes here | Section |
|---|---|---|
| D-01 (ADR 0015) | L9 without the "while in force" hedge; states are place ports; a lowering-computed preset may cover every place | §6.6, §6.8, §14 |
| D-03 (ADR 0017) | "structural" Proved → the `NoOverflowPlace { drop_places }` certificate | §6.5, §14 |
| D-04, D-28, D-41 (ADR 0018), D-47, D-48 (ADR 0022) | the final predicate is copied into deadlock-free; a bad one ends only deadlock-free (and T7 "done") as `PropertyNotCheckable`; `final` in a property stays E407; one name, "final predicate" | §6.8, §10.3, §10.5 |
| D-05, D-25 (ADR 0018) | predicate text opaque in slice 0 with `finalNotChecked`; `[final]` parsed from slice 1, properties from slice 4 | intro, §0 F8, §3.3, §3.4, §6.8, §10.3, §14 |
| D-06, D-07, D-16, D-29, D-31 (ADR 0016) | set-valued async ends; each pair has its own direction; L12 regardless of direction; L7 with at most one async pair; mode ≠ interaction; bare fused names | §0 F5, §6.7, §10.3 |
| D-10, D-11, D-20 (ADR 0019) | predicates hashed as parsed; templates by kind, version and parameters; built-ins seeded explicitly | §2.4, §6.8 |
| D-12, D-19, D-26 (ADR 0020) | lowercase slug IDs; read-only second window; unknown kind isolated to its file; no merge driver, no `.gitattributes`; journals in `aside/` kept; name clashes are model problems | §0 F4, §4.1, §6.1, §10.3, §13.1, §15 |
| D-13 | children = elements whose parent is the scope; `nothingToCheck`; a Person with a `parent` is still a child (conservative call, §6.2) | §2.4, §6.1, §6.2, §10.3, §14 |
| D-14, D-42 | state places compile `overflow`; the own-scope final-state default is derived, never stored | §6.5 |
| D-18 (ADR 0020, ADR 0021) | `suppressions.toml` in ADR 0001's file list; dispositions `accepted` / `not-real`; interaction stated by the user, never set by bindings | §6.7, §8 |
| D-30, D-33 | "assumption" only for relationships; one kind spelling per layer | §6.7, §6.8 |
| D-37 | per-file `FormatVersion`, per-entity conflicts, read-only session are in design §3–§5 | §9.3, §12.1, §13.1 |
| D-70 (ADR 0024), revision 4 | one layout file per element, `layouts/_landscape.toml` removed; every file ends with an empty line; insertion conflicts are rebuilt, not kept on both sides; open time measured at 4,000 files | intro, §0 F1, F9, F12, §1, §2.1–§2.3, §3.3, §3.5, §4.1–§4.4, §6.1, §7, §9.1, §9.2, §10.3–§10.5, §11.1–§11.3, §12.3, §13.3, §13.7, §14, §15, §16, §19 |

Revision 3 changed neither the journal format (§13) nor the canonical writer (§11). Revision 4 changes the canonical writer (the empty last line, §11.1 rule 1; the layout key order, rule 2) and the layout entity keys that journal images use (§9.1, §13.3); the journal's framing and records are unchanged. Format 1 is not yet frozen (§12.5), so none of these changes needs a migration.

### 17.2 Amendments in force

The proposals of revision 2 are decided and recorded; this spec no longer proposes an ADR change.

- **ADR 0001 file list** (revision 2 A1): ADR 0020 adds `suppressions.toml`, `layouts/_landscape.toml` and the per-file `format`. ADR 0024 then replaces the per-view layout files and `layouts/_landscape.toml` with one layout file per element, and adds the empty last line (D-70).
- **NS §2.2 slug narrowing** (A2): ADR 0020 and NS §2.2 revision 3 (D-12).
- **CODING_STANDARDS FMT-1, unknown element kind** (A3) and **FMT-2, temp-file name** (A4): applied in `CODING_STANDARDS.md` (D-36).
- **Design §3–§5** (A5): per-file `FormatVersion` (`Model.versions`), per-entity reload conflicts, `EntityChange` keyed by §9.1, read-only session (D-37, done).
- **NS L11 and SM-B01** (A7): each pair has its own direction (D-07, ADR 0016; NS L11 revision 3).

Follow-ups owned by other documents (D-21), all done (D-55): SM revision 3 §1.5 shows `[state_machine]` with multi-line tables and SM §2.2/SM-B01 read per pair (D-07); PL revision 3 calls a state/event clash a model problem (§4.1), puts F in `DeadlockFree { final }` (§7), stores templates by kind, version and parameters (§9.3), maps property errors to `PropertyNotCheckable` (§10.1) and retires E404/E405 (§10.5).

### 17.3 Glossary terms this spec relies on

**Binding**, **Entity**, **Format version**, **Recovery journal**, **Final predicate**, **Not stated** and **Suppression**, as in the merged glossary block of `consistency-review.md` with D-28 to D-33 (GSS applies them to `GLOSSARY.md`).

## 18. Closed questions

Every open question of revision 2 is decided.

| # | Question (revision 2) | Decision |
|---|---|---|
| Q1 | Reply bindings: one relationship carries a call and its reply, or replies are their own relationships | D-07, ADR 0016: each pair has its own direction; the stored shape did not change, only the `bindingDirection` check (§6.7) |
| Q2 | Lowercase-only IDs for elements, relationships, views and properties | D-12, ADR 0020: yes; hyphens and the `-2` suffix stay (§4.1) |
| Q3 | Second window on a locked project | D-19, ADR 0020: read-only (§13.1) |
| Q4 | Unknown element kind | D-19, ADR 0020: only that file does not load; FMT-1 amended (§6.1, §10.5) |
| Q5 | Per-entity git merge driver | D-19: not in v1; revisit after slice 3 on dogfooding evidence (§15) |
| Q6 | New project writes `.gitattributes` | D-19: no; `*.toml text eol=lf` is documented (§2.4, §15) |
| Q7 | Retention of journals in `aside/` | D-19: all kept in v1 (§13.1) |

Closed before revision 2 and still valid: built-ins are explicit entries (D-20); place ports are never bound (ADR 0016); fused IDs use bare names (D-31); there are no kind defaults for scan attributes (SR §2.2, D-18); a missing place `capacity` is a model problem (L10); load failures are per section (§3.3).

## 19. How this spec was checked (2026-11-02)

**Revision 4 (2026-11-02).** The §14 layout files were rewritten for one file per element (8 layout files, 18 files in all), with the same positions, bend points and behavior layout as revision 3; no element file changed, so the verdicts and the load report stand. The merge claims of revision 4 come from spike E2 (§15), which ran its own canonical writer from §11.1 on a mock of §14 plus one state machine element, with git 2.43.0. All TOML blocks of this document were parsed again with Python 3.12 `tomllib`. Not verified: that toml_edit 0.25 keeps the empty last line when it appends (§11.2 rule 9, a slice 0 test), CRLF files and `core.autocrlf` in merges, and open time at 4,000 files (a slice 0 measurement).

**Revision 3 re-run.** The §14 files were extracted from this document by a script, written to a temp directory and checked again by a throwaway reader and explorer (Python 3.12, `tomllib`, run with `-I`, not in the repo) under the revision 3 rules: L11 per pair in either direction, L12 by bare names, L7 with at most one async pair per port, L4, L5, L9 on the stored predicate and the final predicate, children by `parent`, and deadlock with F = none and with F = `users/signed_in = 4`. Results, all equal to §14: Kinesis 8 places, |R| 330, |O| 21, first overflow at stored 176 after 10 steps on `fe1/peer_threads` (5 of 4), no deadlock, always-reachable `fe1/map_entries = 5` Bad = ∅; Cognito 6 places, |R| 24, |O| 0, deadlock after 3 steps at stored 7 with both values of F; the block variant 5 places, no `overflow` place (so the certificate applies, `drop_places = []`), the same 3-step deadlock with the waiting payload of §14. All 28 TOML blocks of this document parse. The TOML of the §14 files did not change in revision 3, so the merge results of §15 and the toml_edit measurements below stand.

Revision 2 checks:

- **The example project loads.** The 14 files of §14 were written as real files and read by a throwaway Python 3.12 reader (`tomllib`), in a temp directory, not in the repo. The reader applies:
  - the strict key sets of §5–§8;
  - the ID alphabets of §4.1;
  - the references of §10.4;
  - the kind/level table of §6.2;
  - the suppression-key shape of SR §3.7;
  - orphan detection.

  Result: no problems. A script confirmed that the 14 blocks in §14 are byte-identical to those files.
- **The example project gives the stated verdicts.** The same reader lowered both combined scopes by NS §6: bare-name fused IDs, generated places named by the relationship, and L4 and L9 asserted. It explored them with a throwaway explorer written from NS §4, §8.1, §8.2, §8.4, §10.1 and §12.
  - Result: the table in §14. Kinesis: Disproved in 10 steps, stored 176, |R| 330, |O| 21. Cognito: Disproved in 3 steps, stored 7, |R| 24.
  - This equals NS §13.2 and `e5/results.txt` (330 and 24 states).
  - The `kinesis-incident-block` variant gave the stated results; revision 3 names its Proved as the `NoOverflowPlace` certificate.
- **Merge table (§15):** a script ran S1–S13b and S15–S20 with git 2.43.0. S14 is reasoned.
- **toml_edit 0.25.17** (the version `stack.md` names), built in a scratch crate:
  - a no-op render of an LF file with arrays of tables is byte-equal;
  - `value()` assignment drops a trailing comment, and copying the decor keeps it;
  - a CRLF file renders with LF, and re-normalizing to CRLF restores the original bytes;
  - `Table::set_position` takes `Option<isize>`;
  - `ArrayOfTables::insert` places a new `[[property]]` entry between two existing ones, and `ArrayOfTables::retain` removes one, keeping the other entries' comments;
  - conflict markers fail to parse.
- **Journal framing:** a Python prototype of §13.2, with SHA-256 standing in for BLAKE3, recovered the correct prefix at all 361 truncation offsets of a 3-record journal. With one bit flipped, it stopped at that record as damaged.
- **All TOML literals** in this document were parsed with `tomllib`. The journal example's elided `…` hashes are strings, so they parse.
- **Not verified:**
  - load time at about 4,000 files (revision 4; a slice 0 measurement, `docs/product/v1-slices.md`);
  - Windows path lengths, rename retries and `File::try_lock` behavior on Windows (E3);
  - Tauri's `appDirectoriesOverride` behavior (taken from the researcher's citation of the Tauri `PathResolver` docs);
  - the restore algorithm end to end (no session code exists);
  - the scan expectations of §14 (no scan code exists).
