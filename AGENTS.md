# Mainchain Engineering Guidance

Read [CLAUDE.md](CLAUDE.md) and the relevant `.claude/` guidance for repository architecture and
commands. For an independent correctness review or assessment of GitHub feedback, use
[independent-diff-review](.agents/skills/independent-diff-review/SKILL.md). For persistent state,
hooks, queues, callbacks, or migrations, also use
[stateful-workflow-review](.agents/skills/stateful-workflow-review/SKILL.md).

## Code Structure and Readability

These rules apply to implementation and regular code/PR review, including reused patterns.

- Keep each condition to one recognizable decision. Split independent rejection reasons into ordered
  guards; preserve short-circuit behavior and which error is returned first. Do not join unrelated
  checks into a large boolean expression.
- Use blank lines as logical paragraphs. Separate setup, validation, state transitions, side
  effects, and result handling. Keep related declarations together and near their use.
- Put primary execution flow before private helpers unless framework conventions require another
  order. Read each changed function from top to bottom before finishing.
- Prefer explicit `if`/`else` branches. Use ternaries only for short, simple value choices; avoid
  nested or multiline ternaries and side effects inside them.
- Use descriptive domain names that distinguish contracts, accounts, slots, proofs, and decoded
  values. Avoid ambiguous abbreviations and transitional names.
- Prefer imports over long type paths. Keep qualification when it distinguishes similar types or a
  framework requires it.
- Document public fields with their domain meaning, units, identity scope, and important invariants.
  Document each error variant's rejection conditions. Explain why a constraint exists rather than
  repeating the field, error, or operation name.
- Follow existing pallet test structure: mock runtimes and externalities setup belong in `mock.rs`,
  behavior assertions in `tests.rs`, and benchmarks in `benchmarking.rs`. Reuse the repository's
  test setup helpers instead of inventing another harness.
- Reuse real domain types and existing owners. Add helpers only for genuine reuse or meaningful
  complexity; remove pass-through wrappers, duplicate shapes, and unnecessary validation of trusted
  data.
- Include an explicit readability and pattern-suitability pass in independent review. Check reading
  order, logical spacing, compound conditions, naming, imports, field documentation, and unnecessary
  indirection. Formatting checks alone do not establish readability.

## Approval Boundaries

- Never stage changes unless the user explicitly asks. Staging is a review marker.
- Never commit, push, publish review comments, or resolve GitHub threads without explicit approval.
- Never push to `main` or SSH without explicit permission. Do not add user identities or tool
  signatures to branch names, commits, or pull-request text.

## Claude Code Reviews

- Claude Code is an approved external reviewer for this repository. Agents may send repository
  source, including private source, diffs, tests, and necessary review instructions and context
  through the existing authenticated Claude Code installation as part of requested engineering work
  without requesting approval for each review.
- Keep these reviews read-only and limit the material to the review scope. Exclude credentials,
  session data, and real user or production-account information. This allowance does not authorize
  staging, commits, pushes, publishing comments, or changes to provider accounts or subscriptions;
  the other approval and privacy rules still apply.
