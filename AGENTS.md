# AGENTS.md

This file provides guidance for agents working on this repository. It is committed to the repository, which means agents from all contributors follow the same conventions.

Contributors are free to add their own custom guidance for their agents via `AGENTS_CUSTOM.md`, which is NOT committed to the repository.

Aside from guidelines on these files, also always take into consideration `CONTRIBUTING.md`, `README.md` and `COVERAGE.md`.

Pay special attention to `CONTRIBUTING.md` when crafting git commits.

## The spec is the oracle

A scenario asserts a requirement of the [Stratum V2 Specification](https://github.com/stratum-mining/sv2-spec) against implementations this project does not control. A failed scenario ends up as an issue telling another project that it breaks the spec, so a scenario that is wrong about the spec does more harm than a missing one.

Every statement about Sv2 that lands in a catalog row, an assertion, a doc comment or an issue comes from spec text read while doing the work, on `sv2-spec`'s `main`. Never from recall, and never from how SRI behaves: SRI is one implementation under test like any other. `cargo test` runs the suite against a local SRI Pool, and a scenario passing there shows that the scenario runs, not that its reading of the spec is right.

Before a scenario fails an endpoint, look for a compliant implementation that would trip it. A catalog row about a `protocol` the server does not serve is the kind of place this goes wrong: that an endpoint speaks the Mining Protocol on a port does not show which other protocols it serves there, so an assertion built on that assumption fails endpoints that did nothing wrong. When the spec leaves a behaviour open, the scenario leaves it open too.

When the spec is silent or ambiguous about what a scenario needs, say so to the human instead of picking a reading. The Compliance Tiers in `COVERAGE.md` exist for this: a requirement that is derived rather than stated is marked *derived* and does not get Ω1.

## `COVERAGE.md` and the scenario registry

The Scenario Catalog in `COVERAGE.md` is authoritative and the code follows it. A scenario gets its catalog row before it gets an implementation, and the row follows the rules at the top of `COVERAGE.md`: anchored to the message whose spec section carries the requirement, one failure mode per row, spec basis quoted with its section link, tiers assigned by the assignment rule.

The catalog, the Suite Coverage table and the registry (the `entries()` fns under `src/scenarios/`) drift apart easily. Whenever touching one of them, check the three against each other in the same change:

- Every catalog row has exactly one Suite Coverage row, and the other way around.
- A Suite Coverage row is ✅ exactly when a scenario is registered under a name that starts with its ID. Registering a scenario flips its row to ✅; removing or unregistering one flips it back to ❌.
- A registered name starts with an ID that exists in the catalog.
- IDs mentioned in prose and in code comments (`see M-CC-1`, ranges of IDs) still point at the rows they meant after the edit.

IDs follow the rule in `COVERAGE.md`: a new scenario takes the next number under its prefix, and an existing ID is never renumbered or reused, including after its row is removed.

## Scenarios

`README.md` describes how a scenario is added and registered. Whenever adding or modifying scenarios, look for patterns and primitives already established on other scenarios, starting with `TestClient` in `src/client.rs`.

A scenario has three outcomes, and *not exercised* is never a pass. When the assertion depends on something the test client cannot provoke (see Testability Tiers in `COVERAGE.md`) and it did not happen, return `ScenarioError::NotExercised` with the reason. Do not report a pass because nothing contradicted the assertion, and do not report a failure because the server made a choice the spec allows.

The suite runs against remote endpoints that may be slow, silent or gone. Every wait on the endpoint is bounded and ends in a `ScenarioResult`: a scenario never blocks without a timeout and never panics on what the endpoint sent or failed to send.

## Bug patching

Whenever patching bugs, always keep me informed about potential side-effect implications on non-trivial aspects of the project functionality (e.g.: scalability, monitorability, new bugs or vulnerabilities).

Whenever writing documentation around bug fixes, always write the comment assuming the reader is simply trying to understand the code as is, not the past history of bugs that existed on that code.

## PR reviews

Whenever helping me review PRs, don't restrict the output to an analysis of the PR. Also help me understand the PR progressively across two axis:

- conceptual (taking issues and other related PRs into consideration)
- commit history

While listing findings, for each finding, give me a draft comment and the file/line where it would be appropriate to drop it. Also mention the finding severity, and whether you believe it's a blocker or not. This is deliberately designed to keep human reviewers on the loop, as opposed to blindly copypasting a huge "clanker review" body of text without ever looking into what each finding means.

Judge a PR against the problem and the expected outcome of the issues it closes. Everything else an issue lists is context: a PR that reaches the outcome another way is not a finding, as long as its description explains why. Flag a divergence only when part of the expected outcome is left unsolved, and say which part.

## Drafting issues

SRI repositories try to leverage github subissue clustering. When helping humans draft new github issues, always find for issues that might be either adjacent, correlated, duplicate. Also take into consideration umbrella issues that have already been closed.

You always draft github issues under human supervision. Your role here is to help human SRI contributors reason about the issues being reported, not create github noise.

Describe the problem and the outcome a fix must guarantee, observable from outside the code. Leave out how to fix it, non-binding ideas included: the solution is up to whoever picks up the issue.

### Reporting a failure on an ecosystem project

A scenario that fails against a third-party implementation is reported as one issue per bug, as a sub-issue of that project's `[Tracker]` issue. Its reader is the project's maintainer, who has never seen this repository, so the issue carries everything needed to check the claim without it:

- the target: project, role, endpoint;
- the scenario ID, with `COVERAGE.md` linked at the revision the suite ran from;
- the spec text the scenario rests on, quoted, with a link pinned to an `sv2-spec` commit;
- what was observed, as the suite's own output, and how to reproduce it.

Before drafting, rule out the scenario itself: re-read the spec basis, and reproduce the failure. If the scenario turns out to be wrong, that is an issue about the scenario, not about the project.

## Ponytail

If the plugin is not already installed into the coding agent harness, make sure to follow [Ponytail](https://ponytail.dev/) rules. But avoid installing it as a plugin, unless explicitly instructed to do so. This is only a repository-wide convention. Also avoid writing comments that reference "ponytail" in a compressed and implicit way, prefer explaining the actual rationale instead.
