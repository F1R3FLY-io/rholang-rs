---
doc_type: todos
version: "1.0"
last_updated: 2026-11-25
mr_status:
  ready: false
  target_branch: master
---

# Tasks and Epics

<!--
TEMPLATE USAGE INSTRUCTIONS:
0. Update the frontmatter date when modifying this file
   (Update version only for significant structural changes to template)
1. Replace all [PROJECT_NAME] and [PROJECT_SPECIFIC] markers
2. Add new epics using the YAML frontmatter format below
3. Move completed epics to docs/CompletedTasks.md
4. Use /nextTask to find the next task to work on
5. Use /implement to execute tasks with full context
6. Remove these usage instruction comments before committing
-->

This document tracks implementation work through **epics** (logical groupings of related tasks).

**Document Structure**
- Active work: This file (`docs/ToDos.md`)
- User stories: `docs/UserStories.md`
- Completed work: `docs/CompletedTasks.md`
- Backlog: `docs/Backlog.md`

**For LLM assistance in multi-repo workspace:**
See [Task Tracking Standard](https://gitlab.com/smart-assets.io/gitlab-profile/-/blob/master/docs/common/task-tracking-standard.md)

**For reference (GitLab):**
[Task Tracking Standard](https://gitlab.com/smart-assets.io/gitlab-profile/-/blob/master/docs/common/task-tracking-standard.md)

---

## MR/PR Tracking

When all tasks in this file are complete and ready for merge, update the frontmatter:

```yaml
mr_status:
  ready: true
  target_branch: master
  title: "feat: [PROJECT_SPECIFIC: MR title]"
  description: |
    ## Summary
    - [Completed items]

    ## Test plan
    - [x] All tests passing
  labels: ["feature", "enhancement"]
```

---

## Active Epics

<!-- Epics are ordered by priority. Work on the highest priority epic first. -->

---

### EPIC-001: [PROJECT_SPECIFIC: Epic Title]

```yaml
---
epic_id: EPIC-001
title: "[PROJECT_SPECIFIC: Epic Title]"
status: pending          # pending | in_progress | blocked | review | complete
priority: p1             # p0 (critical) | p1 (high) | p2 (medium) | p3 (low)
user_story: US-XXX       # Link to user story in docs/UserStories.md
blocked_by: []           # List of blocking epic IDs
created_at: [DATE]
claimed_by: null         # Implementer ID: human-{email}, {tool}-session[-{id}], or {team}/{role}
claimed_at: null
tasks:
  - id: TASK-001-1
    title: "[PROJECT_SPECIFIC: Task 1 title]"
    status: pending      # pending | in_progress | complete | blocked
    acceptance:
      - "[PROJECT_SPECIFIC: Acceptance criterion 1]"
      - "[PROJECT_SPECIFIC: Acceptance criterion 2]"

  - id: TASK-001-2
    title: "[PROJECT_SPECIFIC: Task 2 title]"
    status: pending
    blocked_by: [TASK-001-1]  # Optional: task dependencies
    acceptance:
      - "[PROJECT_SPECIFIC: Acceptance criterion 1]"
---
```

**Context:** [PROJECT_SPECIFIC: Why is this epic needed? What problem does it solve?]

**Scope:**
- [PROJECT_SPECIFIC: What's included]
- [PROJECT_SPECIFIC: What's explicitly excluded]

**Notes:**
- [PROJECT_SPECIFIC: Implementation notes, gotchas, references]

---

<!-- Add more epics following the same format -->

---

## Epic Template

Use this template when adding new epics:

```yaml
---
epic_id: EPIC-XXX
title: "Short descriptive title"
status: pending
priority: p2
user_story: US-XXX
blocked_by: []
created_at: YYYY-MM-DD
claimed_by: null         # Implementer ID: human-{email}, {tool}-session[-{id}], or {team}/{role}
claimed_at: null
tasks:
  - id: TASK-XXX-1
    title: "Task description"
    status: pending
    acceptance:
      - "Measurable acceptance criterion"
---
```

---

## Task States

| Status | Meaning | Next Action |
|--------|---------|-------------|
| `pending` | Not started | Available to claim |
| `in_progress` | Being worked on | Continue or handoff |
| `blocked` | Waiting on dependency | Check `blocked_by` |
| `complete` | Done | Move to CompletedTasks.md |

The parser accepts only these four values. For a task that waits for review, keep `status: in_progress` and set `review_requested_at` and `review_type`. Move deferred work to `docs/Backlog.md`. Do not use `status: deferred`.

---

## Workflow

1. **Find next task**: Use `/nextTask` to identify the highest priority unclaimed task
2. **Claim task**: Set `claimed_by` using [Implementer Identification](https://gitlab.com/smart-assets.io/gitlab-profile/-/blob/master/docs/common/stigmergic-collaboration.md#implementer-identification) format and `status: in_progress`
3. **Implement**: Use `/implement` to execute with full context
4. **Complete**: Mark `status: complete` when acceptance criteria met
5. **Move epic**: When all tasks complete, move epic to `docs/CompletedTasks.md`

---

## References

- **User Stories:** `docs/UserStories.md`
- **Completed Work:** `docs/CompletedTasks.md`
- **Backlog:** `docs/Backlog.md`
- **MR/PR Tracking Standard:** [docs/common/todos-mr_pr-tracking-standard.md](https://gitlab.com/smart-assets.io/gitlab-profile/-/blob/master/docs/common/todos-mr_pr-tracking-standard.md)
