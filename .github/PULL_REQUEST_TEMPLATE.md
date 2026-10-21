## What and why

<!-- Describe the change and the motivation. Link the issue it resolves. -->

Closes #

## How it was verified

<!--
Not "tests pass". What did you actually run, and what did you observe?
For engine changes: the flow file or CLI invocation you exercised.
For UI changes: what you clicked and what rendered. A screenshot or recording helps.
-->

## Checklist

- [ ] Commits are signed off (`git commit -s`, DCO — see `CONTRIBUTING.md`)
- [ ] `CHANGELOG.md` updated under `[Unreleased]` for user-visible changes
- [ ] New i18n keys added to both `zh-CN` and `en-US`, then `npm run typesafe-i18n`
- [ ] Bug fix includes a test that fails before and passes after (or the PR explains why not)
- [ ] No `unwrap()` in new Rust production paths, no `as any` in new TypeScript
- [ ] Contract changes (flow data model, plugin wire protocol, report schema) were discussed in an issue first
