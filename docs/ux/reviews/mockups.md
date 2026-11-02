# Mockup reviews

## senior UX designer

Preferred: I recommend a combination: Option 2's information architecture and verdict semantics, drawn in Option 1's visual language, plus two pieces from Option 3. Option 2 scores highest overall, so if you want a single option as it stands, pick Option 2.

Why: Option 1 looks the most professional and densest, and its replay is the easiest to read: a ticked scrubber and a Marking table with the previous step, this step and the change as text. But its IA has two serious gaps: the step list is a clipped nested scroll area, and the status bar leaves out Unknown. Option 2 has the IA right: a glossary-correct breadcrumb, a full-height step list that replaces the Verdicts content, a status bar that lists every verdict, Proved evidence with the marking count as slice 0 requires, the most distinct Unknown badge (dashed outline), corner-bracket selection, and the queue glyph on generated places. But its mono-caps texture and dot grid are noisy, and its light replay color is too close to Disproved red. Option 3 is the most spec-faithful in replay behavior, but it is the least dense, uses an editorial identity that works against instrument-grade trust, and contains invented content.

What to build:
- **From Option 1:** neutral gray palette, hairlines, 13px sans with tabular numbers, mono only for ports, places, markings and file paths, the ticked scrubber, the Marking table, and the '1/3 +1' chip.
- **From Option 2:** the breadcrumb 'System landscape › Order platform' with 'Level: Container' separate, the full-height step list during replay, and a status summary written as counts ('3 checks: 1 Disproved, 1 Unknown, 1 Proved') so it never truncates. Also: Proved evidence with the technique and marking count, the dashed-outline Unknown badge, corner-bracket selection, the queue-slot glyph, and the element count in the canvas footer.
- **From Option 3:** the canvas legend, the 'Why' and 'Next step' labels on Unknown, and its 3:1 input border token.

Fixes needed in any version:
- Move the light replay highlight to a yellow-amber (hue about 38–42°) that keeps at least 3:1 on the canvas and is clearly apart from Disproved red.
- Bring the input borders and the pending token up to at least 3:1 (4.5:1 if pending is shown as text).
- Draw the missing states: pending re-evaluation, stale, running with elapsed time and Cancel, Unknown (cancelled), the empty project, and a 300-element level-of-detail view.
- Replace the 'CURRENT' step tag with a non-glossary word or a visual-only marker plus aria-current, and drop revision numbers from the status bar.
- Give the canvas a roving focus with arrow keys instead of one Tab stop per element.

Caveat: the flows spec these mockups cite (§2.3, §2.5, §4.12, §8) is not in the repo, so I checked against GLOSSARY, UX_STANDARDS, design.md and v1-slices only.

Who decides what: the product owner should settle the scope name ('Order platform' vs 'Order System') and whether a word-sense exception is added to GLOSSARY for 'timeout'. senior-frontend-dev should confirm the cost of level-of-detail rendering in React Flow at 300 nodes.

Evidence: screenshots of all states at 1440x900 and 1024x640 in light and dark, with reduced motion, are in /tmp/claude-1000/uxr/shots/. The capture and contrast scripts are /tmp/claude-1000/uxr/run.mjs, contrast.mjs and a11y.mjs. I did not edit any project files.

- Option 1: 7/10. Strengths: The most professional and calm of the three. It uses neutral grays, hairlines instead of cards, 13px system type and tabular figures, so it reads like an IDE or instrument, not a theme. It is also the densest: 3 verdicts are visible at 1440x900, and element cards are about 186x76.; Replay is the easiest to read. The scrubber has a tick for every step and a red end tick. Step-number markers sit on the elements and the relationship. The generated place has a '1/3 +1' chip. The Marking table shows the previous step, this step and the change (▿ −1 / ▵ +1) as text. That serves E7 (root cause) best and does not rely on color.; At step 7 the deadlock explanation names what each side waits for, for example 'Order API waits at send for room in Order API → Billing (publishes to): 3 of 3 messages, when full: block'. This matches E5 case 2 and ADR 0014. The scenario is internally consistent: block means No capacity exceeded is Proved while Deadlock-free is Disproved.; The verdicts are clear. Each shows an icon, the word and a 2px left rule. Unknown is neutral gray with circle-help, a Reason line and 'Next step:', so it never reads as success. Proved carries an honest scope line: 'Time and rates are not modeled.'; Amber is used only for replay, and indigo only for interaction. The token contrasts I measured pass: replay highlight 5.1:1 (light) and 8.9:1 (dark), Unknown 6.1 and 7.4, white on the step marker 5.3.; Findings show the rule ID and name, severity as icon plus word, affected elements as selectable chips, the evidence, and a suppressed finding with its reason (VRD-5).. Problems: Sev 3: the status-bar summary leaves out the Unknown verdict. It reads 'Checks finished: Deadlock-free Disproved, No capacity exceeded Proved.' with no Unknown, so the run looks cleaner than it was. This is the one place where Unknown is quietly dropped.; Sev 3: the step list is a short nested scroll area inside the Disproved row, about 190px tall. At step 4, step 7 (the deadlock explanation, the most important text) is cut off below the fold. Real counterexamples of 20 to 40 steps will be unusable in that box. Keeping Proved and Unknown visible during replay does not justify this; a one-line summary of the other verdicts would.; Sev 3: only the replay scene exists. 'Exit replay' does nothing (focus falls to <body>). There is no edit state, no 'Replay counterexample' entry point, and no idle verdict list, so the non-replay verdict design can't be judged.; Sev 3: inputs fail non-text contrast. --border-default #d1d4d9 is 1.49:1 on the panel in light and 1.43:1 in dark (#30333a). --verdict-pending #c7cbd1 is 1.63:1, so a 'pending re-evaluation' badge would fail as soon as it is drawn.; Sev 2: the Proved evidence says 'Proved with a certificate'. Certificates are cut from slice 0, and the slice 0 acceptance criterion asks for the technique and the number of markings. Option 2's copy does this right.; Sev 2: the breadcrumb 'System Context › Order System › Containers' mixes a level ('Containers', which is not in GLOSSARY) into a path of views. The level already appears as 'Level: Container'.; Sev 2: at 1024x640 the canvas fits to 58%, so element text is about 7px, and the Marking table is clipped in the bottom pane.; Sev 1: the 'CURRENT' tag on the active step reuses the glossary term Current (a verdict computed for the model as it is now) in another sense.; Sev 1: the status bar shows 'Revision 42 · saved at 39'. Revision numbers are an engine detail; the user thinks in terms of saved or unsaved.; Sev 1: in read-only replay the Inspector shows 'Delete element' in full danger red even though it is disabled, so it looks clickable.. UX_STANDARDS: A11Y-1 must: input borders are 1.49:1 (light) and 1.43:1 (dark), below 3:1. The pending verdict token is 1.63:1 and would fail as a badge.; VRD-1 must: the status-bar summary leaves out the Unknown verdict of the third property.; VRD-2 must (partial): the text list of steps is clipped in a nested scroll box, so step 7 is not visible while viewing step 4.; VRD-4 / VRD-6 must / should: no stale, 'pending re-evaluation', running (elapsed time and Cancel) or 'Unknown (cancelled)' state is drawn.; TXT-1 must: 'Containers' in the breadcrumb is not a glossary term. 'CURRENT' collides with the glossary term Current.; CNV-1 must: the breadcrumb mixes a level name into the view path instead of the GLOSSARY form (System landscape › <system>), with the level shown separately.; WIN-1 must: at 1024x640 the Marking table and the step list are clipped. The canvas text at 58% is not readable.; CNV-4 / A11Y-3: density at 300 elements is not shown (no level-of-detail or zoomed-out state). Every canvas element is its own Tab stop, where a roving focus is needed.
- Option 2: 8/10. Strengths: It follows GLOSSARY and the spec most closely. The breadcrumb is 'System landscape › Order platform' with 'Level: Container' shown separately. Element IDs are not shown. The status bar names all three verdicts. The Proved evidence reads 'Checked all 1,970 reachable markings; re-checked by a second explorer', which meets the slice 0 acceptance criterion (technique plus number of markings).; Unknown is the most clearly distinct of the three options: a neutral gray, a question-mark icon, the word, and a dashed outline on the badge, with a Reason and a Next step. Its reason copy is honest: 'Budget exhausted while re-checking … A Proved needs both explorers to agree, so this is Unknown.'; The replay IA is the best. The step list replaces the Verdicts content at full height, so all 7 steps and the deadlock explanation are visible without nested scrolling. The Inspector becomes read-only. 'Replay counterexample' is the only verdict-level primary action, and it appears only on Disproved.; The generated place is drawn as a queue glyph on the relationship, with filled slots and a '2/3' chip. That is the fastest way to see that the queue is filling (E7).; Corner-bracket selection can't be confused with the focus ring or the replay highlight. Elements outside the step are not dimmed. Port names hide below 100% zoom, which is a sensible first step toward semantic zoom.; The title block (VIEW, LEVEL, ELEMENTS, ZOOM) gives a persistent element count, which is useful for the 300-element target, and puts zoom where the user expects it.; The dark palette is well separated, and the dark replay highlight is 10.3:1. The findings table separates suppressed findings into their own group with the reason.. Problems: Sev 3: in the light theme the replay highlight #b04a06 (hue about 24°) is close to Disproved #b8282b (hue about 359°). On the canvas the involved elements read as an error red, and users with protanopia or deuteranopia will merge the two. That undermines the claim that the highlight is never a verdict color. Move it toward a yellow-amber (hue about 38–42°) that still has at least 3:1 on the canvas.; Sev 3: inputs fail non-text contrast. --border-default #b3c0d1 is 1.8:1 on the panel in light and 1.69:1 in dark. --verdict-pending is 2.23:1 (light) and 2.5:1 (dark).; Sev 2: at 1024x640 the status-bar summary is cut off at 'Can always get back to Billing', so the word Unknown is the part that disappears. Use counts instead, for example '3 checks: 1 Disproved, 1 Unknown, 1 Proved', which never truncates a verdict word.; Sev 2: there is a lot of visual noise. Mono caps markings appear on every element kind, every section header, the breadcrumb ('CTX', 'CON') and the title block, on top of a dot grid with major dots. That adds texture without adding information, and at 300 elements it competes with the names. The 'CTX'/'CON' abbreviations also ask the user to recall jargon.; Sev 2: the title block sits inside the canvas at the bottom and will cover elements in dense views. It needs to collapse or avoid content.; Sev 2: while replay is open, the Proved and Unknown verdicts are not visible at all. A one-line summary or 'Other properties (2)' row would keep them in mind (recognition over recall).; Sev 1: the status bar shows 'Unsaved changes since rev 139 · Revision 142', which exposes revision numbers and the abbreviation 'rev'. The step list uses a 'CURRENT' tag that collides with the glossary term Current.; Sev 1: the step copy 'Step 2 of 7: Billing: take_lock' is terser than Option 1's 'Order API → Orders DB (writes to): lock_row', which carries the relationship context.. UX_STANDARDS: A11Y-1 must: input borders are 1.8:1 (light) and 1.69:1 (dark). The pending verdict token is 2.2–2.5:1.; WIN-1 must / VRD-1 must: at 1024x640 the status-bar summary truncates, and the Unknown verdict word is what gets cut.; VRD-4 / VRD-6 must / should: no stale, 'pending re-evaluation', running (elapsed time and Cancel) or 'Unknown (cancelled)' state is drawn.; TXT-1 must (needs a GLOSSARY decision): 'timeout' in REL-NO-TIMEOUT ('Synchronous call without a timeout') is an Avoid word for Budget. GLOSSARY itself uses 'unknown timeout' under Not checkable, so add an explicit word-sense note. Separately, 'CURRENT' collides with the glossary term Current.; CNV-4 / A11Y-3: density at 300 elements is not shown. Canvas elements are div role=button, each with tabindex=0 (6 Tab stops), where a roving focus is needed.
- Option 3: 5/10. Strengths: It follows the replay spec carefully. The 36px read-only bar has First, Previous, the scrubber, Next and Last plus 'Step 4 of 7'. The step list is a listbox where selection follows focus. A Start row, 'Follow steps' and an 'Order API | Billing' switch are included, the waiting places are outlined at the last step, and the full-block deadlock says what it waits for.; It is the only option with a canvas legend ('In this step', '- - Asynchronous'). That teaches the highlight and edge encoding without relying on color alone.; Unknown carries labeled 'WHY' and 'NEXT STEP' sections, which is the clearest structure for VRD-3.; Its input borders are the only ones that pass: --border-control #8B8273 is 3.58:1. Light and dark are separate warm palettes, and the dark replay ochre is 10.2:1.; The status bar announces replay entry with keyboard help ('Use ← and → to step').. Problems: Sev 3: the density is the lowest of the three. The 14px body at 1.55 line height, serif headings, large padding and the 'Container view / Order System' caption block mean that only about 1.5 verdicts fit in the panel at 1440x900. The caption sits on the canvas and will collide with elements at 300 per view. Serif element names also lose legibility at the zoom levels a 300-element view needs (about 7px at the 1024 fit).; Sev 3: the editorial look works against 'precision instrument' trust. Warm paper and serif read like documentation or a publication rather than a tool whose verdicts must be trusted. Choosing it would also mean deciding on a brand and visual identity.; Sev 2: at 1024x640 the status-bar summary is cut off at '…lock_b = 1 Unk…', so the Unknown verdict word is truncated. The breadcrumb truncates to 'System Cont…', which weakens CNV-1.; Sev 2: the property title 'Can always get back to Billing: lock_b = 1' exposes raw predicate syntax and an internal place name. Option 2's 'Can always get back to Billing: idle' shows the readable form.; Sev 2: the finding rule REL-CALLBACK is invented. It is not in the slice 5 rule set (ARCH-CYCLE, RUN-SYNC-CYCLE, REL-NO-TIMEOUT, DATA-SHARED-DB, REL-SPOF).; Sev 2: the order is inconsistent. The verdict list shows Disproved, Unknown, Proved and the rationale says this matches the header summary, but the status bar reads Disproved, Proved, Unknown.; Sev 2: there are two ink-filled primary buttons in view (Check in the header and Replay counterexample in the panel). In dark mode the ink primary inverts to a cream button that looks like a secondary surface.; Sev 1: the breadcrumb uses 'Containers' (not a glossary term), and the step list uses a 'CURRENT' tag. The highlighted elements are tinted fills plus an outline and a 7px ring, which is heavier than needed and will merge in dense views.. UX_STANDARDS: WIN-1 must / VRD-1 must: at 1024x640 the status-bar summary truncates the Unknown verdict word, and the breadcrumb is truncated.; CNV-1 must: the breadcrumb truncates at the minimum width and puts 'Containers' (a level) in the path of views.; TXT-1 must: 'Containers' is not a glossary term. 'CURRENT' collides with the glossary term Current. The 'lock_b = 1' predicate is shown raw instead of in plain language (TXT-3 should).; VRD-5 must (content): the finding uses a rule (REL-CALLBACK) that the scan does not have.; VRD-4 / VRD-6 must / should: no stale, 'pending re-evaluation', running (elapsed time and Cancel) or 'Unknown (cancelled)' state is drawn. --verdict-pending #8B8273 is 3.58:1, which is enough for a badge but fails 4.5:1 if used as text.; CNV-4 / A11Y-3: density at 300 elements is not shown, and the caption overlay will collide with content. Every canvas element is its own Tab stop.

## product manager

Preferred: Option 2 "Blueprint" as the base, toned down with Option 1's restraint, plus two parts from Option 1. Scores: 8, 7, 5.

Why Option 2. It is the only option that follows the spec and slice 0 where it matters for the v1 gate:
- Replay replaces the Verdicts content, so the step 7 deadlock explanation is visible without scrolling.
- Proved names the technique and the number of markings, as the slice 0 acceptance criterion asks.
- The Unknown reason explains the second-explorer rule (ADR 0008).
- The rule IDs exist in the slice 5 set, and the header has no slice 1 undo or redo.
- It has the clearest separation between verdicts (the dashed gray Unknown box), and the queue-slot glyph makes the generated place readable during replay (E7, J4).

What to take from Option 1:
- The replay Marking table with 'Step n−1 | Step n | Change' columns and the ticked scrubber. They make the per-step cause readable.
- Its compact type scale and hairline restraint. Cut the mono caps down to markings, port names and capacity chips; drop the 'CTX'/'CON' breadcrumb prefixes; replace the title block with plain zoom and fit controls; defer the corner brackets.

Option 3 is out. It does not fit an instrument-like tool or the 300-element density target, and it has the WIN-1 clipping problems listed above.

Decisions only the product owner can make:
1. The scope's name: 'Order platform' (the spec) or 'Order System'.
2. The breadcrumb form. I recommend 'System landscape › Order platform' plus a 'Level: Container' chip: it uses GLOSSARY words and still meets CNV-1.
3. Whether replay replaces the verdict list (spec, Options 2 and 3) or keeps the other verdicts visible (Option 1's deviation). I recommend replacing it.
4. A glossary note or lint exception for 'timeout' in its network sense (REL-NO-TIMEOUT).

Fixes for whichever option wins:
- Stop labeling the active step 'CURRENT' (all three do). Current is a GLOSSARY verdict term.
- Make the status-bar summary list every verdict, including Unknown.

Before freezing the tokens, add these frames to the chosen mockup (cheapest test, about a day):
- The states not yet drawn: pending re-evaluation (design.md:131), running with elapsed time and cancel, Unknown 'cancelled', stale (slice 1), empty and error.
- A seeded 300-element view at fit zoom, to define the zoomed-out level of detail. No option tested this.

Pass rule for these frames: pending and Unknown are never mistaken for Proved in light or dark, and element names stay legible at the zoom where 300 elements fit.

Two more things to fix:
- The flows spec that all three cite (§2.5, §4.3, §4.12, §8) is not in the repo; docs/ux/ holds only the mockups and is untracked. Commit it so this review can be checked against it.
- Strip the slice 1–5 extras (undo and redo, breadcrumb, Findings tab, user-property verdicts) from the slice 0 build. The visual direction is a two-way door; don't let it widen slice 0 scope or pull presentation polish ahead of the v1 gate.

Screenshots (both themes, 1440 × 900 and 1024 × 640): /tmp/claude-1000/mk/. For Option 2 there are both the verdicts view and the replay preset; for Option 3, both the replay and the edit scene.

- Option 1: 7/10. Strengths: Looks the most like a calm, professional engineering tool (Instruments/Linear register): 1px hairlines, 13px system type, tabular figures. It is the most compact of the three, which makes it the best starting point for 300 elements per view. At 1024 × 640 the canvas drops to 58 % and stays legible (screenshots in /tmp/claude-1000/mk/o1-*.png).; Clear accent discipline. Indigo is used only for interaction and selection, and amber only for replay. At step 4, Order API is outlined in amber while Billing is 'Selected' in indigo, and the two never get confused, in light or dark.; Has the best per-step explanation for E7 (root cause from replay): the Marking table with 'Step 3 | Step 4 | Change' columns and ▽ −1 / △ +1 as text, the '1/3 +1' chip on the generated place, a ticked scrubber, and numbered step markers on the element and the relationship. The deadlock cause can be read off the steps: step 1 Order API takes lock_row, steps 2–6 fill the queue, step 7 Billing waits for row_lock.; Proved carries the E5 caveat 'Holds for every order of steps … Time and rates are not modeled.' Unknown gives the reason (budget exhausted, 1,000,000 markings in 5.0 s) and a 'Next step:'. The note that a run that stops counts as a deadlock until final markings exist matches slice 0, where final markings come in slice 4.; Contrast holds on the key pairs (muted text on canvas 5.8:1, element border 4.6:1, replay amber 5.1:1).. Problems: Deviation (a) undermines the main job of replay. The step list sits inside the Disproved row, so at 1440 × 900 step 7 ('Deadlock: no step is possible' plus the two waiting lines, i.e. the root cause) is scrolled out of view, and at 1024 × 640 only steps 3–6 fit. Keeping Proved and Unknown visible during replay helps no job: during replay the user is answering E7. Take the spec behavior: the step list replaces the Verdicts content.; The status bar says 'Checks finished: Deadlock-free Disproved, No capacity exceeded Proved.' and leaves out the third check, which ended Unknown (option-1-precision.html:916). A summary that quietly drops the Unknown is the kind of thing that makes Unknown read as nothing happened.; Proved says 'Proved with a certificate'. Certificates are cut from slice 0 (docs/design/design.md:178: 'no stubborn sets or certificates'), and the slice 0 acceptance criterion asks the panel to name 'exhaustive exploration, number of markings' (docs/product/v1-slices.md).; The rule ID REL-SHARED-DB does not exist. The slice 5 rule set uses DATA-SHARED-DB (v1-slices.md, slice 5).; Undo and redo appear in the header, but they are slice 1 (slice 0 'Out'). Breadcrumb, Findings tab and the 'Never more than 2…' user property are slices 3, 5 and 4. That is fine for a direction study, but they must not leak into the slice 0 build.; None of the pending re-evaluation, running (elapsed time, cancel), stale, empty or error states is drawn, so the direction has not been tested where Unknown or pending could pass for success (design.md:131).. UX_STANDARDS: TXT-1 must: the 'CURRENT' tag on the active replay step reuses the GLOSSARY term Current (a verdict computed for the behavior the model now describes). It will collide with the current, pending and stale verdict markers. Use 'This step' or aria-current only.; TXT-1 must: the breadcrumb 'System Context › Order System › Containers' uses 'Containers' and 'Order System', and neither is a glossary term or view name. GLOSSARY: a view is the inside of an element or the system landscape.; VRD-1 should (spirit): the status-bar summary of finished checks leaves out the Unknown verdict.; VRD-5 must: the rule ID REL-SHARED-DB is not a defined rule (it should be DATA-SHARED-DB).
- Option 2: 8/10. Strengths: Follows the flows spec and slice 0 most closely. The step list replaces the Verdicts content during replay, so step 7 with both waiting lines is fully visible at 1440 × 900. 'Replay counterexample' is the only primary button and it opens on Start. The Inspector is read-only during replay. Slice 0 has no undo and redo, and the header shows none. Breadcrumb 'System landscape › Order platform' plus 'Level: Container' uses GLOSSARY words and still meets CNV-1.; Its verdicts are the clearest of the three. Each one is a boxed badge with its own icon and word: Disproved in red, Proved in green, Unknown as a dashed neutral gray outline. At a glance the dashed Unknown cannot be mistaken for Proved (VRD-1).; Its evidence copy is the most accurate. Proved says 'Checked all 1,970 reachable markings; re-checked by a second explorer', which meets the slice 0 acceptance criterion and ADR 0008. Unknown says 'Budget exhausted while re-checking: the second explorer stopped … A Proved needs both explorers to agree, so this is Unknown.' That reason teaches the ADR 0008 rule exactly where the user meets it. Assumptions are listed per relationship that binds no ports.; It shows the generated place most legibly in replay: the queue is drawn as 3 slots with 2 filled, plus a '2/3' chip on the relationship. This makes J4 (sizing a capacity) and the overflow and back-pressure story readable at a glance. Elements outside the step are not dimmed (border-quiet stays 3.1:1).; Its findings table matches slice 5: rule IDs from the defined set (DATA-SHARED-DB, REL-NO-TIMEOUT), severity, affected elements as selectable chips, evidence, and a suppressed finding that stays listed with its reason (VRD-5).; It has a distinct identity without looking retro, and its navy dark theme is the most polished of the three.. Problems: Too many mono caps. Kind labels, breadcrumb prefixes ('CTX', 'CON'), the title block (VIEW / LEVEL / ELEMENTS / ZOOM), section headers and the dot grid together add visual noise. Keep mono for markings, port names and capacity chips only, and drop the breadcrumb prefixes.; The drawing-sheet title block and the corner-bracket selection are decoration that costs build time. Under the north-star non-goal 'presentation polish' they wait. The title block is the zoom control, so slice 0 needs just zoom plus fit.; The scenario's root cause cannot be named from the replay alone, which is the E7 risk. Step 2 has Billing take lock_b, and step 7 says 'Billing waits at take_lock for a token in lock_b' without saying who holds it, so it reads like a self-deadlock. Step 4 fires two unrelated transitions in one step ('Order API: send, Billing: await_payment'), and an asynchronous binding does not fuse them. These are content errors, not direction errors, but the replay mockup is what we judge replay readability on.; Element cards are the tallest of the three (kind, name, tech line, port labels), which hurts the 300-elements-per-view target. Port names hide below 100 % zoom, but there is no zoomed-out level-of-detail design.; Like all three options, it draws no pending re-evaluation, running, stale, empty or error state.. UX_STANDARDS: TXT-1 must: the 'CURRENT' tag on replay step 4 reuses the GLOSSARY term Current.; TXT-1 must: the rule name 'Synchronous call without a timeout' (REL-NO-TIMEOUT) uses 'timeout', which GLOSSARY lists as an Avoid word for Budget. Here it has the network sense, which is legitimate, but it needs an explicit lint exception or a glossary note, not silent use.; TXT-1 should: the 'CTX' and 'CON' breadcrumb prefixes and 'rev 139' are abbreviations that are not in the glossary.
- Option 3: 5/10. Strengths: It is the most attractive and the calmest to read. Hierarchy comes from type, and the 14px body at 1.55 line height reads best in the verdict prose.; Unknown always shows WHY and NEXT STEP under small labels and sits between Disproved and Proved. It is the most thorough VRD-3 layout.; Its ochre 'highlighter' fill on involved elements is the most visible replay highlight at a distance. Its deadlock explanation names both waits, and the callback scenario (Order API's only worker blocked on a full queue while Billing calls back to confirm) is the most realistic root cause for E7.; It follows the spec on replay: the step list replaces Verdicts and the Behavior model tab opens with 'Follow steps'.. Problems: It reads as a reading app, not an engineering instrument. Serif element names on the canvas get poor at low zoom. The generous padding is the opposite of what 300 elements per view needs, and the designer concedes that at 1024 × 640 the canvas text is 'very small'.; In the warm palette, highlighted and quiet elements are separated less clearly. In dark mode the brown tint on the cards looks muddy.; At 1440 × 900 the replay scene cuts Orders DB off behind the bottom pane, and the step 7 explanation is below the fold. At 1024 × 640 the 'Fit' control in the behavior-model toolbar is clipped.; 'Proved with a certificate' is not a slice 0 evidence type (design.md:178), and slice 0 asks for exhaustive exploration plus the number of markings.; REL-CALLBACK, rated High, is an invented rule that is not in the slice 5 set (ARCH-CYCLE, RUN-SYNC-CYCLE, REL-NO-TIMEOUT, DATA-SHARED-DB, REL-SPOF). Undo and redo are shown, though they are slice 1.; Like all three options, it draws no pending re-evaluation, running, stale, empty or error state.. UX_STANDARDS: WIN-1 must: 'nothing is cut off' fails. Orders DB is clipped by the bottom pane in the replay scene at 1440 × 900, and the Fit control is clipped at 1024 × 640.; TXT-1 must: the 'CURRENT' tag on the replay step reuses the GLOSSARY term Current. The breadcrumb uses 'Containers' and 'Order System', which are not glossary terms.; VRD-5 must: finding REL-CALLBACK names a rule that does not exist in the rule set.

## Mockups

- Option 1 Precision instrument: `/home/nils/.t3/projects/petrinet/docs/ux/mockups/option-1-precision.html`
  - 1. Precision instrument: cool neutral grays, 1px hairlines instead of cards, 13px system type with tabular figures, and mono for ports, file paths and markings. Indigo is the only interactive accent (Check, focus, selection). Amber is reserved for replay highlights, so a replay highlight is never confused with focus or selection.
2. Verdicts read like an instrument dial: icon plus word right-aligned on the scope line, with a 2px rule in the verdict's color on the left edge. Unknown uses the neutral gray with circle-help and has a "Next step:" line. The deadlock waiting lines, +1/−1 changes and capacity chips are all text, so nothing depends on color.
3. Replay is shown at step 4 of 7 in all its places: the replay bar with a ticked scrubber, the involved element and relationship in amber with step-number markers, a "1/3 +1" chip on the generated place, the behavior model pane, and a per-step Marking table. Stepping updates all of them, and each step change produces one polite announcement.
4. Two deviations to put to the product owner: (a) the step list sits inside the Disproved row instead of replacing the Verdicts tab, so Proved and Unknown stay visible at 1440×900; (b) the breadcrumb and Findings tab (S3/S5) are drawn because the brief asked for them, and the scope is named "Order System" (spec: "Order platform") to match the breadcrumb.
5. The scenario is internally consistent with ADR 0014. The queue is set to block, so No capacity exceeded is Proved (certificate) while Deadlock-free is Disproved (the Order API holds the row lock while it fills the queue, and Billing waits for that lock). The Never-more-than property is Unknown with budget exhausted. Light/dark is a control outside the app frame, and URL hashes such as #dark,findings,step7 open any state.

  Tokens:

```
Semantic color tokens, light / dark (components use only these; no hex outside the two token blocks):
--surface-app #f3f4f6 / #0d0e10
--surface-panel #ffffff / #151619
--surface-canvas #f9fafb / #101113
--surface-raised #ffffff / #1b1c20
--surface-sunken #f1f2f4 / #0b0c0e
--surface-hover #e9ebee / #22242a
--surface-selected #e8edfc / #1c2444
--text-primary #15171a / #ebecef
--text-secondary #464c55 / #b1b6be
--text-muted #5c636d / #9298a1
--text-disabled #8d939b / #5d626a
--text-on-accent #ffffff / #ffffff
--border-hairline #e2e4e8 / #24262b
--border-default #d1d4d9 / #30333a
--border-strong #6c727b / #868c95
--border-quiet #8a9099 / #686e77
--edge #6c727b / #868c95
--canvas-grid #dadde2 / #25272c
--accent #3550c8 / #4560e0
--accent-hover #2b43ad / #3a52c4
--accent-subtle #e8edfc / #1c2444
--accent-text #2f48b8 / #9aabff
--focus-ring #3550c8 / #8fa1ff
--verdict-proved #1c7047 / #4bc48a
--verdict-disproved #c22f2a / #ff6b61
--verdict-unknown #5c636d / #a0a6af (neutral gray; circle-help icon; never a check mark)
--verdict-stale #767c85 / #7e848d
--verdict-pending #c7cbd1 / #34373d
--replay-highlight #a85400 / #f0a23a
--replay-highlight-ring #fcefdc / #3a2913
--problem #b23f0e / #f08c4e
--elev-1 0 1px 0 rgb(16 18 22/.04), 0 1px 2px rgb(16 18 22/.06) / 0 1px 0 rgb(0 0 0/.35)
--elev-2 0 1px 3px rgb(16 18 22/.08), 0 6px 16px rgb(16 18 22/.10) / 0 0 0 1px rgb(255 255 255/.04), 0 8px 24px rgb(0 0 0/.55)

Tokens shared by both themes:
Type: --font-sans (system-ui, Segoe UI, Noto Sans, Liberation Sans, …); --font-mono (ui-monospace, Cascadia Mono, SF Mono, DejaVu Sans Mono, …); --fs-xs 11/16, --fs-sm 12/16, --fs-base 13/20, --fs-md 14/20, --fs-lg 15/22; --fw-regular 400, --fw-medium 500, --fw-semibold 600; --tracking-caps .06em; tabular figures everywhere.
Spacing (4 px grid): --sp-half 2, --sp-1 4, --sp-1h 6, --sp-2 8, --sp-2h 10, --sp-3 12, --sp-4 16, --sp-5 20, --sp-6 24, --sp-8 32 (px).
Radius: --radius-sm 3, --radius-md 5, --radius-lg 8, --radius-pill 999 (px).
Lines: --hairline 1px, --stroke-edge 1.25px, --stroke-highlight 2.5px, --focus-width 2px.
Layout (flows spec §2.3): --header-h 44, --status-h 24, --replaybar-h 36, --tabstrip-h 32, --right-w 360 (320 under 1280 px wide), --bottom-h 264 (240 under 700 px tall), --target-min 24 (px).

Verification:
- Contrast: a script reads every token pair from the file. All text pairs are at least 4.5:1, and all borders, edges, focus ring, verdict icons and replay highlight are at least 3:1, in both themes. 0 failures. Lowest: --border-quiet on the canvas, 3.08 (light) and 3.68 (dark).
- Screenshots from headless Chromium: replay at 1440×900 light and dark, step 7 (deadlock), Findings tab, Inspector tab, and 1024×640.
- Script errors: none in the console.
- Network: no external URLs in the file.
- Copy lint against GLOSSARY Avoid words: no wrong uses (the only matches are the spec's own "bound" and the rule name "Single point of failure").
- Not verified: a manual keyboard-only pass and a screen-reader pass. The Python Playwright driver is not installed, and npm is blocked by the proxy (407).
```

- Option 2 Blueprint: `/home/nils/.t3/projects/petrinet/docs/ux/mockups/option-2-blueprint.html`
  - 1. Blueprint identity: a cool blue-slate palette over a dot grid with a stronger dot every 5 steps, small 2–6 px radii and thin precise borders. Monospace caps are used only for markings (element kinds, port names as pin labels, the "capacity 3 · block" chip, rule IDs, file paths, revisions, step numbers). The canvas controls sit in a drawing-sheet title block (VIEW / LEVEL / ELEMENTS / ZOOM), and the selection uses drafting corner brackets. That keeps it distinct from a generic shadcn look without looking retro.
2. Verdicts are never color-only. Each one has its own icon, the word and a 3 px tone rule. Unknown uses a neutral gray from the gray ramp with the question-mark icon and a dashed outline, so it can't read as success. Disproved alone gets the primary "Replay counterexample" button. Replay uses a separate amber highlight with numbered markers, uses no opacity on the elements not involved, and every token pair meets AA.
3. I followed the flows spec over the literal task wording in four places, and each one needs your call:
   - Breadcrumb: the task asked for "System Context › Order System › Containers". The mockup uses the §4.12 and GLOSSARY form instead: "System landscape › Order platform" plus "Level: Container". "Order System" and "Containers" are not terms in the glossary.
   - Element IDs: the "monospace for IDs" brief is applied to markings only, because the spec says the UI never shows an ID such as `Billing/take_lock`.
   - Unsaved state: the header carries the save state as §2.5 says. The status bar adds the revision pair ("Unsaved changes since rev 139 · Revision 142").
   - Theme toggle: it sits in the mockup strip, not in the app, because the app follows the OS theme.
4. To match the IA, the requested states are presets of one window instead of one crammed frame. Replay replaces the Verdicts content with the step list, opens the bottom pane on Behavior model and makes the Inspector read-only. "Replay counterexample" opens on Start as the spec says, and the preset jumps to step 4.
5. Slice gaps and invented content:
   - The breadcrumb (S3), the Findings tab (S5) and the third, template-based property (S4) are rendered because the task asked for them, though the spec says not to render them before their slice.
   - Copy I wrote myself: the counterexample steps, the finding texts, and the extra status-bar and header strings.
   - REL-NO-TIMEOUT uses "timeout" in its network sense. The copy lint will need a word-sense exception for it, because GLOSSARY lists "timeout" as an Avoid word for Budget.
   - Open question for the UX designer: should the third property be a built-in property in S0–S2 captures instead?

  Tokens:

```
I built the Blueprint mockup as one self-contained 80 KB HTML file. It has no network requests (I grepped for URLs) and no web fonts. All icons are inline lucide-style SVG, with no cloud provider logos.

A strip at the top, labeled as not part of the app, switches between three states and two themes: Edit · Verdicts (the default), Edit · Inspector, and Replay · step 4 of 7. The same states load from the URL with #verdicts, #inspector or #replay, plus #light or #dark. The app itself follows the OS theme as the spec says, so the toggle is only there for review.

What the mockup shows:
- **Header:** the project name `order-platform` with "Unsaved changes", the breadcrumb, "Level: Container", Save, Reload, Check (the only primary button) and Commands.
- **Canvas:** 6 elements (Customer, Web app, Order API, Billing, Orders database, Payment provider) and 6 labeled relationships. Ports appear as small square handles with their names in monospace, like pin labels on a schematic. Order API → Billing (publishes to) carries "capacity 3 · block", and its generated place is drawn as a 3-slot glyph on the line.
- **Selection:** Billing is selected, drawn with drafting corner brackets. The inspector follows §4.3, and in replay every field is read-only under "Exit replay to edit".
- **Verdicts:** Deadlock-free is Disproved, with the deadlock lines, the no-final-markings caveat and "Replay counterexample". No capacity exceeded is Proved, with evidence, assumptions and scope. "Can always get back to Billing: idle" is Unknown because the budget ran out during the re-check, with the next step.
- **Replay at step 4 of 7:** an amber replay bar with First, Previous, a scrubber, Next and Last. Order API, Billing and the generated place ("2/3") are outlined with numbered markers. The step list is a listbox with a Current marker. The behavior model pane shows Order API with −1 and +1 change markers. Stepping actually works through the arrow keys, the buttons, the scrubber and the list.
- **Findings tab:** DATA-SHARED-DB (Medium) and a suppressed REL-NO-TIMEOUT that shows its reason and "Remove suppression".
- **Status bar:** the combined "Checks finished: …" message on the left, "Unsaved changes since rev 139 · Revision 142" on the right.

**Design tokens (light / dark)**

*Type*
- --font-sans: system-ui, -apple-system, "Segoe UI", "Noto Sans", "Helvetica Neue", Arial, sans-serif (both themes)
- --font-mono: ui-monospace, SFMono-Regular, "Cascadia Mono", "JetBrains Mono", "DejaVu Sans Mono", Menlo, Consolas, monospace (both)
- Sizes: --fs-2xs 10.5px · --fs-xs 11px · --fs-sm 12px · --fs-md 13px (body) · --fs-lg 14px · --fs-xl 16px · --fs-2xl 18px
- --lh-tight 1.25 · --lh-body 1.45
- Weights: 400 / 500 / 600
- --tracking-marking 0.08em

*Spacing*
- --space-1…9: 2 / 4 / 6 / 8 / 12 / 16 / 20 / 24 / 32 px

*Radius*
- --radius-xs 2px · -sm 3px · -md 4px · -lg 6px · -pill 999px

*Layout*
- --header-h 44 · --status-h 24 · --right-w 360 (320 under 1280 wide) · --bottom-h 280 (240 under 760 tall) · --replaybar-h 36 · --grid-step 16

*Surfaces*
- --surface-app: #e6ebf2 / #050b14
- --surface-canvas: #f3f6fa / #0a1320
- --surface-panel: #fbfcfd / #0e1828
- --surface-raised: #ffffff / #132134
- --surface-sunken: #edf1f6 / #0b1422
- --surface-hover: #e4eaf3 / #182a42
- --surface-selected: #dce7f8 / #1a3253
- --canvas-dot: #bccadb / #213650
- --canvas-dot-major: #9fb1c8 / #2e4a6c

*Text*
- --text-primary: #0e1a2b / #e4ebf5
- --text-secondary: #33455d / #b6c3d4
- --text-muted: #52637b / #8e9fb6
- --text-on-accent: #ffffff / #06111f

*Borders and lines*
- --border-subtle: #d6dee9 / #1d2d44
- --border-default: #b3c0d1 / #2c405d
- --border-strong: #5f7189 / #7189aa
- --border-quiet: #7e8ea4 / #566d8d
- --edge: #5a6c85 / #8196b3
- --port: #1f57c3 / #7eb0ff

*Accent and focus*
- --accent: #1f57c3 / #7eb0ff
- --accent-hover: #1a479f / #a4c7ff
- --accent-subtle: #e2ebfa / #13294a
- --accent-text: #1a4fb3 / #8dbaff
- --focus-ring: #1f57c3 / #8dbaff

*Verdicts*
- --verdict-proved: #13744a / #5fd29a; -bg: #e4f3ea / #0e2a20
- --verdict-disproved: #b8282b / #ff8f86; -bg: #fbeaea / #33171b
- --verdict-unknown: #5b616b / #a9afb8 (neutral gray); -bg: #eef0f2 / #1a1f27
- --verdict-stale: #676d77 / #858b95
- --verdict-pending: #a3acb8 / #4b596c

*Replay and other states*
- --replay-highlight: #b04a06 / #f7b545
- --replay-highlight-ring: rgba(176,74,6,.18) / rgba(247,181,69,.22)
- --replay-highlight-bg: #fdf0e3 / #34270f
- --problem: #9a3f0c / #ffa46e
- --severity-medium: #85520a / #e9b85c
- --danger: #b8282b / #ff8f86

*Elevation*
- --elev-1: 0 1px 2px rgba(18,36,66,.08) / 0 1px 2px rgba(0,0,0,.45)
- --elev-2: 0 1px 2px rgba(18,36,66,.06), 0 3px 8px rgba(18,36,66,.08) / 0 1px 2px rgba(0,0,0,.4), 0 4px 10px rgba(0,0,0,.35)
- --elev-3: 0 10px 28px rgba(18,36,66,.18), 0 2px 6px rgba(18,36,66,.08) / 0 12px 32px rgba(0,0,0,.55), 0 2px 6px rgba(0,0,0,.4)

**Verification**
- **Contrast:** a script read the token values from the file and checked 41 color pairs in both themes. All of them pass:
  - Text is at least 4.5:1; muted text is 5.4 or more.
  - Element borders, lines, ports, the focus ring, the replay highlight and the verdict icons are at least 3:1. Borders of elements not involved in the replay step come out at 3.08 (light) and 3.52 (dark).
  - --verdict-stale failed at 4.09 in light; I changed it to #676d77, which gives 5.07.
- **Screenshots:** I captured the page with the local headless Chromium at 1440×900 (Verdicts light, Inspector dark, Replay light and dark) and at 1024×640 (Replay light, Verdicts light and dark).
  - At 1024×640 during replay, the canvas, the step list and the behavior model pane are all visible.
  - The header keeps Check, "Unsaved" and the level visible, as §2.5 requires at that width.
  - In edit mode the bottom pane drops to its tab strip under 700 px of height, per §2.3. Because of that, the Findings tab only shows its contents at 1440×900.
  - The drawing scales to fit the canvas, and port names hide below 100 % zoom except on the selected element.
- **JavaScript:** a DOM dump confirmed the scripts ran (replay mode on, the scrubber's text value set, the dark theme applied). No console errors appeared.

**Not verified**
- Keyboard and screen-reader behavior beyond reading the markup: no Orca pass and no axe run.
- Forced-colors mode beyond one small CSS block.

**Things to know**
- `git status` lists untracked dotfiles at the repo root (.bashrc, .profile, .gitconfig and others) that I did not create. They look like files left behind by the sandbox and should not be committed.
- I did not commit anything.
```

- Option 3 Calm editorial: `/home/nils/.t3/projects/petrinet/docs/ux/mockups/option-3-editorial.html`
  - 1. Editorial means hierarchy through type rather than chrome: serif headings for project, element and property names, small tracked uppercase labels ("CONTAINER", "EVIDENCE", "WHY", "NEXT STEP"), a 14 px body at 1.55 line height, and hairline dividers instead of boxed panels. The canvas opens with a "Container view / Order System" caption, like a figure in a book.
2. Warm paper and deep ink are separate palettes, not inversions. The one primary action is ink-filled (Check, Replay counterexample). Replay uses an ochre "highlighter" (2 px outline, tint, numbered step marker) so it never reads as a verdict color, and opacity is never used for elements outside the step.
3. Verdicts are never shown by color alone: each has an icon, a word and a left rule. Unknown uses the neutral gray with circle-help, always gives "Why" and "Next step", and sits between Disproved and Proved, the same order as the header summary.
4. The replay follows the flows spec: a 36 px read-only bar with First/Previous/scrubber/Next/Last and "Step 4 of 7", and a Start row plus 7 steps in a listbox where selection follows focus. The behavior model pane shows counts with ▲ +1 / ▼ −1, outlines the transition of the current step, and has "Follow steps" plus an "Order API | Billing" switch. The last step outlines the waiting places, and the full-block deadlock says what it waits for (E5 case 2).
5. Two places where I chose between the task and the spec. First, the breadcrumb reads "System Context › Order System › Containers" with "Level: Container" as the task asked, and the scope is named "Order System"; the spec writes it as "Order platform", so the product owner should pick one name. Second, the property panel and the replay step list share the right column's Verdicts tab, so the mockup has a scene switch instead of showing both at once; the edit scene shows the Findings tab, and replay opens the Behavior model tab as the spec requires. At 1024 × 640 the header collapses in the spec order, and the canvas is scaled to fit, so its text is very small at that size.

  Tokens:

```
I built and checked the mockup. It is one self-contained file with inline CSS, a small inline JS block and an inline SVG icon sprite (generic lucide-style shapes, no cloud logos). It makes no network requests and uses no web fonts.

What I checked, at 1440 × 900 and 1024 × 640, in light and dark (driven through the cached headless Chromium):
- No console errors and no requests leave the file.
- Keyboard: Enter on "Replay counterexample" moves focus to the current step row, ↑/↓ change the step, Home goes to "Start", → on the canvas announces the step once, and Escape exits replay with focus back on the button.
- Contrast is computed from the shipped token values: 0 failures (text ≥ 4.5:1; borders, edges, focus and selection ≥ 3:1).
- Visible copy has no GLOSSARY "Avoid" words.

The page opens on the OS theme. A small "Mockup controls" strip above the window switches the theme and the scene ("Replay at step 4 of 7" / "Edit: verdicts and findings").

Fonts: display is a serif system stack (Iowan Old Style, Charter, … Noto Serif, Georgia), UI is system-ui, mono is ui-monospace.

Shared tokens (same in both themes):
- Type: --fs-2xs 11px · --fs-xs 12 · --fs-sm 13 · --fs-md 14 · --fs-lg 16 · --fs-xl 18 · --fs-2xl 24; --lh-tight 1.25, --lh-body 1.55; --tracking-kicker 0.08em
- Spacing: --sp-1 4 · --sp-2 8 · --sp-3 12 · --sp-4 16 · --sp-5 20 · --sp-6 24 · --sp-8 32 · --sp-10 40
- Radius: --r-sm 5 · --r-md 8 · --r-lg 12 · --r-xl 18 · --r-pill 999
- Layout: --h-header 44 · --h-status 24 · --h-replay 36 · --w-right 360 (320 below 1280 wide) · --h-bottom 280 (240 below 700 tall)

Color tokens (light / dark):

| Token | Light | Dark |
|---|---|---|
| --surface-app | #F6F2EA | #121419 |
| --surface-panel | #FBF8F2 | #171A20 |
| --surface-raised | #FFFDF9 | #1E2229 |
| --surface-canvas | #EFE9DE | #0E1014 |
| --surface-hover | #F1EBDF | #22262E |
| --surface-selected | #E9E1D2 | #2A2F38 |
| --canvas-dot | #D8CFBE | #262A32 |
| --text-primary | #1E1B16 | #ECE6DA |
| --text-secondary | #4F4940 | #BDB6A8 |
| --text-muted | #6B6458 | #9D968A |
| --text-on-accent | #FBF8F2 | #14161B |
| --border-subtle | #E2DACB | #2A2E36 |
| --border-control | #8B8273 | #7C776D |
| --border-element | #7A7266 | #8A8478 |
| --border-element-quiet | #8B8273 | #7C776D |
| --edge | #7A7266 | #8A8478 |
| --accent | #1F2D4A | #E8E1D3 |
| --accent-hover | #2C3D61 | #F5EFE3 |
| --focus-ring | #2457C5 | #8DB4FF |
| --selection | #1F2D4A | #B9C8EA |
| --verdict-proved | #2C6A3F | #7CC28D |
| --verdict-disproved | #A3342A | #F2907F |
| --verdict-unknown (neutral gray) | #5E5A53 | #ABA59B |
| --verdict-stale | #6F695F | #8F897F |
| --verdict-pending | #8B8273 | #7C776D |
| --replay-highlight | #93540C | #E7B565 |
| --replay-highlight-ring | #F1D9AE | #4A3A1C |
| --replay-tint | #F6E3BF | #3A2E17 |
| --on-replay-highlight | #FFFDF9 | #14161B |
| --problem | #9B3B1F | #F0A070 |
| --danger | #A3342A | #F2907F |

Elevation (light / dark):

| Token | Light | Dark |
|---|---|---|
| --shadow-1 | 0 1px 0 rgb(70 50 20/.05), 0 1px 3px rgb(70 50 20/.08) | 0 1px 0 rgb(0 0 0/.35), 0 1px 3px rgb(0 0 0/.35) |
| --shadow-2 | 0 1px 2px rgb(70 50 20/.06), 0 6px 18px rgb(70 50 20/.10) | 0 2px 4px rgb(0 0 0/.35), 0 8px 24px rgb(0 0 0/.45) |
| --shadow-inset | inset 0 1px 0 rgb(255 255 255/.6) | inset 0 1px 0 rgb(255 255 255/.03) |
```
