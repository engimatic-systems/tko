---
name: tko-auditor
description: Read-only audit workflow for tko ticket graphs before handoff, large implementation, dependency sequencing, or context compaction. Use when asked to audit a tko graph, validate whether an epic entry leads to the right first task, surface ambiguities in tickets, or check whether ticket text is structurally self-sufficient before work begins.
---

# TKO Auditor

Use this skill to test whether a tko ticket graph can stand on its own.

The auditor is read-only. It may suggest ticket edits or missing information,
but it must not modify tickets, source files, services, state, or environment
config.

## Workflow

1. Read local project instructions first:
   - `AGENTS.md` or equivalent agent instructions
   - ticket conventions, if the project has them
   - the target ticket with `tko show --full <id>`
   - immediately related tickets only as needed
2. Inspect graph shape with `tko query`, `tko ready`, and `tko blocked`.
3. If an independent second pass is available and the user explicitly wants one,
   run it read-only with minimized context.
4. Do not pass the conversation's intended answer, hidden sequencing theory, or
   suspected defects to the auditor. Pass only repo path, conventions path, and
   target ticket id(s).
5. Compare the auditor's inferred plan against the current graph.
6. Report structural findings before implementation starts.

## Subagent Prompt Template

Use a prompt like this, adjusted only for paths and ticket ids:

```text
In <repo>, inspect the repository conventions and ticket <ticket-id> only.
Do not edit files. Do not use prior conversation assumptions.
Plan the sequence of work needed to accomplish <ticket-id>.
Return: likely work steps, files likely touched/created, risks, verification
commands, and any questions/ambiguities. Keep it concrete.
```

If auditing an epic, ask for the inferred first concrete task and why.

## Report Format

Return concise sections:

- **Entry Point**: epic/task id and what it appears to mean.
- **Inferred Sequence**: work order implied by tickets and deps.
- **Ready/Blocked**: which tickets are actually actionable.
- **Ambiguities**: missing decisions, vague acceptance criteria, unclear paths,
  false dependencies, or hidden assumptions.
- **Suggested Ticket Changes**: edits to scope, deps, notes, or acceptance
  criteria. Suggestions only unless the user asks to modify tickets.
- **Implementation Guardrails**: verification commands and surfaces to inspect
  before closing tickets.

## Signals

Treat these as graph defects:

- A child ticket depends on its own parent epic only to encode membership.
- A granular ticket names an implementation step rather than a durable review
  surface.
- A ticket cannot be planned without conversation history.
- Acceptance criteria close by intent rather than evidence.
- The inferred first task differs from the intended first task.
- Multiple generated or derived files can diverge without a source-of-truth rule.

Treat these as useful output, not failure:

- The auditor asks concrete questions.
- The auditor chooses a different first task.
- The auditor finds likely files or commands missing from the ticket.
- The auditor identifies a dependency that should be sequencing preference, not
  a hard dependency.
