---
doc_type: user_flows
version: "1.0"
---

# User Flows

Detailed user interaction patterns for this project. Each flow captures the
journey, ordered steps, key interactions (which become integration test
assertions), and success metrics (which become test budgets). Flows are
linked bidirectionally to user stories (`docs/UserStories.md`) and to
epics (`docs/ToDos.md`).

**Document Structure**
- User flows: this file (`docs/User-Flows.md`)
- User stories: `docs/UserStories.md`
- Implementation tracking: `docs/ToDos.md` (epics + tasks)

**Linkage**
- A flow lists `Related Stories` and an `Implemented in` epic
- Stories carry a `User Flow:` back-reference
- Epic YAML carries a `user_flow:` field
- Integration tests appear on the flow as `Integration Tests:` once authored
  (late-stage: written after the implementation epic completes and unit
  tests pass). Run `test-spec FLOW-XXX` to seed the artifact and `link
  FLOW-XXX <ITEST-NNN|path>` for subsequent additions.

---

## Personas

Add reusable persona definitions here (Name + one-line description) and
reference them by name from each flow's `Personas:` field.

---

## Core Workflows

<!-- Created flows are inserted above the "Planned Flows" section below. -->

---

## Planned Flows

(Empty)

---

## Related Documentation

- [User Stories](UserStories.md)
- [ToDos](ToDos.md)
