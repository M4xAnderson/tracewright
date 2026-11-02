# Behavior is combined by binding ports of sibling elements and flattening

An element has at most one behavior model, either a Petri net or a state machine, and both are part of the metamodel and the file format from the start. A behavior model offers ports (places or transitions). A relationship between two sibling elements in one view binds their ports: a synchronous binding fuses two transitions (each caller of a port gets its own fused transition), an asynchronous binding connects them through a generated place with a capacity. A state machine is compiled to a Petri net (one place per state, one token), and its events are its ports, so the checker only ever sees Petri nets. To check combined behavior, the sibling nets are flattened into one net, with a map from every place and transition back to its element so a counterexample can be replayed on the architecture.

## Considered Options

- **Bind relationships directly to transitions inside another element's net:** simpler, but renaming an internal transition would break files of other elements and rule out later contract abstraction.
- **Refinement relations between levels in v1:** the strongest model, but too much for one developer before the basic loop works.
- **Petri nets only, state machines later:** rejected by the user because retrofitting a second notation into ports, file format and replay would be messy.

## Consequences

- v1 composes only siblings inside one view; there is no composition across levels.
- An asynchronous place is a bag: message order and request/reply matching are lost. Properties that need them must wait for colored nets or an export to TLA+.
- v1 state machines are flat (no nested states) and have no data variables, so they compile to 1-safe nets.
