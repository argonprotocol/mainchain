# Mainchain Engineering Guidance

Read [CLAUDE.md](CLAUDE.md) and the relevant `.claude/` guidance for repository architecture and commands. For an independent correctness review or assessment of GitHub feedback, use [independent-diff-review](.agents/skills/independent-diff-review/SKILL.md).

## Approval Boundaries

- Never stage changes unless the user explicitly asks. Staging is a review marker.
- Never commit, push, publish review comments, or resolve GitHub threads without explicit approval.
- Never push to `main` or SSH without explicit permission. Do not add user identities or tool signatures to branch names, commits, or pull-request text.

## Claude Code Reviews

- Claude Code is an approved external reviewer for this repository. Agents may send repository source, including private source, diffs, tests, and necessary review instructions and context through the existing authenticated Claude Code installation as part of requested engineering work without requesting approval for each review.
- Keep these reviews read-only and limit the material to the review scope. Exclude credentials, session data, and real user or production-account information. This allowance does not authorize staging, commits, pushes, publishing comments, or changes to provider accounts or subscriptions; the other approval and privacy rules still apply.
