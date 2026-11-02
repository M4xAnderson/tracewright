# Visual direction: recommendation

> **Decided 2026-11-02 by the product owner:** the combination: Option 2's information architecture and verdict semantics in Option 1's visual language, plus the parts from Option 3 listed below, with the fixes in section 3.
>
> **Baseline, 2026-11-02:** `mockups/option-2b-drafting-instrument.html` implements this combination (quiet chrome, monospace only for identifiers, amber replay trace, queue slots, step copy that names who holds what). The product owner approved its look; later UI work starts from it.
>
> **Font, 2026-11-02:** the app bundles IBM Plex Sans and IBM Plex Mono (SIL Open Font License) so it renders the same on WebKitGTK, WebView2 and WebKit. Mono is used only for identifiers (ports, places, markings, file paths).

**Pick Option 1's visual language on Option 2's information architecture, and add three small parts from Option 3.** If you would rather approve one file as it stands, pick Option 2 and apply the fixes in section 3. Drop Option 3.

Both reviews reach this result independently. Where they disagree, I make the call in section 2. The visual direction is a two-way door: it must not widen slice 0 or delay the v1 gate, which is finding a real deadlock or overflow in a design you drew.

## 1. The three options

**Option 1 "Precision instrument" (7/10)**
- **Suits:** daily long sessions with dense views. It reads like an IDE or measuring tool: neutral grays, hairlines, 13 px system type, tabular figures. It is the densest option, the closest to the 300-elements-per-view target, and the calmest to look at.
- **Best part:** replay explains the root cause best (E7). It has the ticked scrubber, step markers on elements and relationships, the "1/3 +1" chip, and a Marking table with "Step n−1 | Step n | Change" as text.
- **Risks:**
  - The step list is a clipped box about 190 px tall inside the Disproved row. Step 7, the deadlock explanation, is out of view.
  - The status bar drops the Unknown verdict, which breaks VRD-1.
  - Input borders are 1.5:1 and the pending token is 1.6:1, which breaks A11Y-1.
  - Copy error: the rule ID REL-SHARED-DB (the real one is DATA-SHARED-DB, D-18). ("Proved with a certificate" is correct after all: the `NoOverflowPlace` certificate ships in slice 0, D-03; use the flows `V-PRO-1` wording.)
  - Only the replay scene exists.

**Option 2 "Blueprint" (8/10)**
- **Suits:** whoever wants the most spec-faithful starting point and a visible identity.
- **Best parts:**
  - The breadcrumb uses glossary words, and replay replaces the Verdicts content at full height.
  - Proved names the technique and the number of markings (slice 0). The Unknown reason teaches the two-explorer rule (ADR 0008).
  - It has the most distinct Unknown (dashed gray outline), the queue-slot glyph on generated places, and corner-bracket selection.
- **Risks:**
  - It is noisy: mono caps on every label, a dot grid with major dots, "CTX"/"CON" prefixes, and a title block that will cover elements in dense views.
  - The light replay amber #b04a06 (hue about 24°) reads as Disproved red.
  - Input borders are 1.8:1. The element cards are the tallest of the three.
  - The replay content is wrong: step 7 does not say who holds lock_b, and step 4 fuses two unrelated transitions. This is the frame E7 readability is judged on.

**Option 3 "Calm editorial" (5/10)**
- **Suits:** reading verdict prose. It is the most attractive at rest, and it is the only option with a canvas legend and labeled "Why" / "Next step" sections.
- **Risks:**
  - About 1.5 verdicts fit at 1440×900.
  - Serif names fail at the zoom where 300 elements fit.
  - A warm paper look sets a brand rather than signalling an instrument you can trust.
  - It has clipping at both window sizes (WIN-1), an invented rule (REL-CALLBACK; a synchronous call back is RUN-SYNC-CYCLE), and two primary buttons.

## 2. Recommended combination

| Take | From | Why |
|---|---|---|
| Neutral palette, hairlines, 13 px sans with tabular figures, mono only for ports, places, markings, capacity chips and file paths | 1 | Density and trust; the least noise at 300 elements |
| Ticked scrubber, Marking table, "1/3 +1" chip, step markers on relationships | 1 | E7: the cause of each step can be read as text |
| Breadcrumb "System landscape › Order platform" with "Level: Container" shown separately | 2 | Glossary terms; a view path is not a level path |
| Step list replaces the Verdicts content at full height; Inspector read-only during replay | 2 | VRD-2: the deadlock step is never clipped |
| Proved evidence copy (technique and marking count), Unknown reason copy, dashed-outline Unknown badge | 2 | Slice 0 acceptance criterion, ADR 0008, VRD-1 |
| Queue-slot glyph on the generated place | 2 | Fastest way to see a queue fill (J4, ADR 0014) |
| Corner-bracket selection | 2 | Selection, focus and replay must be three different signals. It costs four pseudo-elements, which is cheap. |
| Canvas legend (collapsible), "Why" / "Next step" labels on Unknown | 3 | Teaches the encoding; VRD-3 |

**How I settled the points where the reviews disagree:**
- **Title block:** drop it. Use plain zoom and fit controls at the bottom right, and put the element count in the status bar.
- **Corner brackets:** keep them. The product manager wanted to defer them, but without them selection and focus are both indigo outlines and look the same.
- **Other verdicts during replay:** keep one non-interactive row above the step list, "Other properties: 1 Proved, 1 Unknown". This replaces Option 1's deviation and keeps the full-height list.

## 3. Problems to fix in whichever option is chosen

1. **A11Y-1, must:**
   - Split borders into `--border-hairline` (dividers only, decorative) and `--border-control` (inputs, element outlines, at least 3:1).
   - Render pending as text at 4.5:1 or more.
   - Move the light replay highlight to hue 38–42°.
   - Never draw Disproved red on the canvas, so the replay highlight cannot be confused with it.
2. **VRD-1 and WIN-1, must:** write the status summary as counts, "3 checks: 1 Disproved, 1 Unknown, 1 Proved", so a verdict word is never truncated or dropped.
3. **TXT-1, must:**
   - Remove the "CURRENT" step tag. Use a filled marker plus `aria-current="step"`. Current is a glossary verdict term.
   - Remove "rev 139 / Revision 142". Show only saved or unsaved.
   - Remove "Containers" and "CTX"/"CON" from the breadcrumb.
   - Add a word-sense note to the glossary for "timeout" in REL-NO-TIMEOUT.
   - Use only the rule IDs of `docs/specs/scan-rules.md`: ARCH-CYCLE, RUN-SYNC-CYCLE, REL-NO-TIMEOUT, DATA-SHARED-DB, REL-SPOF. Replace REL-SHARED-DB with DATA-SHARED-DB (Option 1) and REL-CALLBACK with RUN-SYNC-CYCLE (Option 3); never invent an ID for a mockup.
4. **VRD-4 and VRD-6, must and should:** draw the missing states before freezing the tokens:
   - pending re-evaluation (design.md:131)
   - stale (slice 1)
   - running, with elapsed time and Cancel
   - Unknown (cancelled)
   - empty project, load or save error, and a check that is not checkable
5. **CNV-4 and A11Y-3:**
   - Draw a seeded 300-element view at fit zoom with a defined level of detail. Below about 60 % zoom, show the name only: no tech line and no port names.
   - Give the canvas one Tab stop with roving focus and arrow keys.
6. **E7 content:**
   - Every waiting line names who holds the resource ("Billing waits at take_lock for lock_b, held by Order API since step 1").
   - Each step fires one transition or one fused transition.
   - Step copy carries the relationship ("Order API → Orders database (writes to): lock_row").
7. **Disabled controls during replay:** "Delete element" uses muted text, not danger red.
8. **Slice discipline:** the breadcrumb (S3), Findings (S5), user properties (S4) and undo/redo (S1) stay out of the slice 0 build.
9. **UX_STANDARDS conflict:** CNV-1's example lists levels as the breadcrumb. Change it to "path of views (System landscape › …) plus the current level", so the rule matches the glossary.

## 4. Proposed tokens

Each pair is light / dark. The table keeps Option 1's semantic set; rows marked * are changed or new. I checked contrast with the WCAG formula against panel, canvas, sunken, selected and hover surfaces. Text pairs reach at least 4.5:1, and borders, focus and the highlight reach at least 3:1, unless a row says otherwise.

| Token | Light | Dark | Use / lowest ratio |
|---|---|---|---|
| --surface-app / -panel / -canvas | #f3f4f6 / #ffffff / #f9fafb | #0d0e10 / #151619 / #101113 | |
| --surface-raised / -sunken | #ffffff / #f1f2f4 | #1b1c20 / #0b0c0e | |
| --surface-hover / -selected | #e9ebee / #e8edfc | #22242a / #1c2444 | |
| --text-primary | #15171a | #ebecef | 15.0 / 12.8 |
| --text-secondary | #464c55 | #b1b6be | 7.3 / 7.4 |
| --text-muted | #5c636d | #9298a1 | 5.1 / 5.2 |
| --text-disabled | #8d939b | #5d626a | disabled controls only (contrast exempt) |
| --text-on-accent | #ffffff | #ffffff | 6.7 / 5.3 on --accent |
| --border-hairline | #e2e4e8 | #24262b | dividers only, never the only boundary of a control |
| --border-control * | #7d838c | #6f757e | inputs, elements: 3.8 / 3.9 on panel |
| --border-quiet | #8a9099 | #686e77 | elements not in the replay step: 3.1 / 3.7 on canvas |
| --edge | #6c727b | #868c95 | relationships: 4.6 / 5.3 |
| --canvas-grid | #dadde2 | #25272c | plain dots, no major dots |
| --accent / -hover | #3550c8 / #2b43ad | #4560e0 / #3a52c4 | Check, primary, selection brackets |
| --accent-subtle / -text | #e8edfc / #2f48b8 | #1c2444 / #9aabff | text 6.4 / 7.0 |
| --focus-ring | #3550c8 | #8fa1ff | 2 px, 2 px offset: 6.0 / 7.1 |
| --verdict-proved | #1c7047 | #4bc48a | check icon, solid rule: 4.8+ / 6.9 |
| --verdict-disproved | #c22f2a | #ff6b61 | x icon, solid rule; panel only: 4.7 / 5.4 |
| --verdict-unknown | #5c636d | #a0a6af | circle-help, dashed outline: 5.1 / 6.2 |
| --verdict-stale * | #62686f | #8a9099 | history icon plus "Stale": 4.7 / 4.7 |
| --verdict-pending * | #5c636d | #9298a1 | clock icon plus "Pending re-evaluation", no outline: 5.1 / 5.2 |
| --replay-highlight * | #8a5a00 | #e8b03a | 2.5 px stroke, hue 39°/41° (Disproved is about 2–4°): 5.0 / 7.7 |
| --replay-tint * | #fdf3d8 | #33270f | fill behind involved elements, decorative |
| --on-replay-marker * | #ffffff | #15171a | step number on the marker: 5.9 / 9.2 |
| --problem | #b23f0e | #f08c4e | finding severity, always with icon and word |
| --elev-1 / -2 | Option 1 values | Option 1 values | popovers and menus only; panels use hairlines |

**Shared tokens (Option 1, unchanged):**
- Type: system-ui and ui-monospace stacks; sizes 11/12/13/14/15 px (13 is body); weights 400/500/600; tabular figures.
- Spacing: 4 px grid (2, 4, 6, 8, 10, 12, 16, 20, 24, 32).
- Radius: 3/5/8/999 px.
- Strokes: 1 / 1.25 / 2.5 px; focus 2 px.
- Layout: header 44, status 24, replay bar 36, tab strip 32, right panel 360 (320 under 1280 px wide), bottom panel 264 (240 under 700 px tall).
- Motion: replay steps change instantly with reduced motion; otherwise a 120 ms crossfade, no movement.

## 5. How to check this works

- **Frames first:** add the frames from fixes 4 and 5 to the combined mockup. That is about a day.
- **Pass rule for the frames:**
  - Pending and Unknown are never read as Proved, in light or dark.
  - Element names are readable at the zoom where 300 elements fit.
  - Step 7 is visible at 1024×640 without nested scrolling.
- **Self-test (E7):** replay the deadlock in two of your own designs. You should name the root cause from the step list and Marking table alone, within 2 minutes, without opening the behavior model pane.
- **What I did not check:** I computed contrast for the new and changed pairs only. Nothing is rendered and there was no screen-reader pass, so re-run the screenshot and contrast scripts on the combined file.

## 6. Decisions for you

1. **Scope name:** "Order platform" (the spec) or "Order System". I recommend "Order platform".
2. **"timeout":** add a word-sense exception for REL-NO-TIMEOUT to the glossary.
3. **Flows spec:** commit it. §2.3, §2.5, §4.12 and §8 are cited by every mockup but are not in the repo.
4. **Unknown badge:** accept the dashed Unknown outline as the one place where outline style carries meaning.

The mockups reviewed are in `/home/nils/.t3/projects/petrinet/docs/ux/mockups/` (`option-1-precision.html`, `option-2-blueprint.html`, `option-3-editorial.html`).