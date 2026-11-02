# A property belongs to a scope element and is a linear marking predicate

A property is stored with the element that is its scope, either for that element's own behavior model or for the combined behavior of the children in its view, because a property about combined behavior cannot belong to any single child. It refers to places through qualified `element/port` IDs. Besides built-in properties (deadlock-free, no capacity exceeded, no dead transitions), user properties are linear constraints over markings (a weighted sum of places compared with a constant) combined with and, or and not, stated as "never" or "always reachable". Initial and final markings are model data. The language is kept this small because every property must be decidable on bounded nets and provable with invariant or integer-programming certificates.

## Consequences

Properties like "eventually" (liveness) and temporal logic beyond reachability are not expressible in v1.
