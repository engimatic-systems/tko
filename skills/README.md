# Example skills

These are **example agent skills** for prospective tko users — illustrations of
how you might teach an AI coding agent (Claude Code, Codex, and similar) to use
tko in your own projects. They are not required to use tko, and they are not
normative: copy one into your agent's skills directory and adapt it to your own
conventions. Installation paths and packaging differ by agent tool; treat these
as source examples, not a universal installer.

Each skill is a directory with a `SKILL.md` — YAML frontmatter (`name`,
`description`) plus a Markdown body the agent reads on demand.

- `tko-basics/` — first contact: the mental model and the minimal
  create → work → evidence → close loop.
- `tko-ticket-discipline/` — ticket editing discipline: readable Org,
  scoped tickets, evidence notes, and lint before close.
- `tko-auditor/` — read-only graph audit before handoff, large implementation,
  dependency sequencing, or context compaction.
