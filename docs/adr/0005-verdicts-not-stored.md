# Verdicts are recomputed, not stored, and identified by a content hash

A check is identified by a hash of the flattened net and the property, built from IDs and excluding layout and display names. Verdicts are kept in an in-memory cache keyed by that hash and are not written to the project, so a stored verdict can never claim more than the current model supports; checks run again when a project opens. A verdict is stale when the current hash differs from the one it was computed for, which needs no dependency tracking, and undo brings earlier verdicts back from the cache. A running check is cancelled only when an edit changes its hash, so layout changes never cancel checks. Storing verdicts later requires a certificate that the tool re-checks on load.

Amended by ADR 0013.
