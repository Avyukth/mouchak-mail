# Agent Communication Protocol (Worker + Reviewer)

## Goals

- Keep the worker focused with minimal disruption.
- Keep the reviewer current with fresh, high-signal context.
- Minimize network overhead and message volume.

## Cadence (Low-Chatter Default)

1. **[TASK_STARTED]** once per task.
2. **[CHECKPOINT]** optional, only if task > 60 minutes or high risk.
3. **[COMPLETION]** once per task with full summary and verification.

Target: 2-3 messages per task.

## Message Templates

### [TASK_STARTED]
Subject: `[TASK_STARTED] <task-id>: <title>`
Body:
- Context (2-4 sentences)
- Acceptance criteria (bullets)
- Expected files/areas

### [CHECKPOINT] (Optional)
Subject: `[CHECKPOINT] <task-id>: <title>`
Body:
- Progress summary (2-4 sentences)
- Any risks or blockers
- ETA to completion

### [COMPLETION]
Subject: `[COMPLETION] <task-id>: <title>`
Body:
- What changed (short bullets)
- Why (short bullets)
- Key files touched
- Risks/edge cases
- Verification steps and results
- Follow-ups or beads links

## Fresh-Context Handoff

- Always include a one-screen summary in [COMPLETION].
- Use a single thread_id per task (e.g., `TASK-<id>`).
- Prefer `summarize_thread` over extra messages.

## Non-Disruption Rules

- Worker does not wait on review unless a hard dependency exists.
- Reviewer does not request mid-task updates unless a blocker is found.
- Silence is normal; no need to "heartbeat" messages.

## Network Overhead Controls

- Batch updates into a single message where possible.
- Avoid new threads; append to existing thread_id.
- Avoid large attachments; use links or paths.
