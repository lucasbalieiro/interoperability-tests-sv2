# Contributing to Sv2 Interoperability Tests

First off, thanks for taking the time to contribute! ❤️

All types of contributions are encouraged and valued. Please make sure to read the relevant section before making your contribution. It will make it a lot easier for us maintainers and smooth out the experience for all involved. The community looks forward to your contributions. 🎉

## I Have a Question

> If you want to ask a question, we assume that you have read the documentation available at [stratumprotocol.org/docs](https://stratumprotocol.org).

Best way to ask a question is to hop onto our community [Discord](https://discord.com/invite/fsEW23wFYs). Two most suitable places to post a question are:
- #newbies-qs and
- #dev, for technical questions, suitable for developers building on top of SRI, or contributing to it.

If you then still feel the need to ask a question and need clarification, we recommend the following:

- Open an [Issue](https://github.com/stratum-mining/interoperability-tests-sv2/issues/new).
- Provide as much context as you can about what you're running into.

We will then take care of the issue as soon as possible.

## What Should I Know Before I Get Started

### Important Resources

In order to have a better overview about what this project covers, have a look at the following resources before getting started with contributions.

  - Stratum V2 Protocol [Specifications](https://github.com/stratum-mining/sv2-spec).
    - Every scenario on this repository tests a requirement of the spec, so this is the text contributions are checked against.
  - [`COVERAGE.md`](COVERAGE.md).
    - The catalog of scenarios, the rules they follow, and the table of which ones are implemented.
  - [`README.md`](README.md).
    - How to run the suite, and how a scenario is added.
  - [SV2 Applications](https://github.com/stratum-mining/sv2-apps).
    - The suite is built on top of its `integration_tests_sv2` crate, and `cargo test` runs it against its Pool.

### Project Communications

Most project communications happen in our [Discord](https://discord.gg/fsEW23wFYs) server. Communications related to general development typically happen under [dev](https://discord.com/channels/950687892169195530/958814900770205739) channel.

Discussion about specific codebase work happens in GitHub [issues](https://github.com/stratum-mining/interoperability-tests-sv2/issues/) and on [pull requests](https://github.com/stratum-mining/interoperability-tests-sv2/pulls/). For discussions about the protocol itself, see the [Specifications](https://github.com/stratum-mining/sv2-spec) repository.

Our dev calls are scheduled every Tuesday at 16:00 UTC. You can see them in the sidebar under Events on Discord and subscribe to them to be notified.

## I Want To Contribute
> When contributing to this project, you must agree that you have authored 100% of the content, that you have the necessary rights to the content and that the content you contribute may be provided under the project license.

### Contribution workflow

The SRI project follows an open contributor model, where anyone is welcome to contribute through reviews, documentation, testing, and patches. Follow these steps to contribute:

1. **Fork the Repository**

2. **Create a Branch**

3. **Make Your Changes**

    These guidelines should be kept in mind:
    - When touching Rust code, make sure the changes have corresponding Rustdocs. Be concise and avoid unnecessary verbosity.
    - When adding or modifying features, check whether some corresponding documentation on .md files needs to be updated accordingly.
    - A scenario needs a row in the Scenario Catalog of `COVERAGE.md` before it is implemented. Once it is implemented, its row in the Suite Coverage table goes from ❌ to ✅ in the same PR.
    - Make sure to run `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings` and `cargo +nightly fmt --all` on your changes. Formatting is checked on nightly `rustfmt`, everything else on the toolchain pinned in `rust-toolchain.toml`.

4. **Commit Your Changes**

    These guidelines should be kept in mind:
    - Progressive commit history, with clear separation of concerns.
    - Avoid individual commits that address specific review findings, which breaks commit history cohesion. Always fold review findings into the original commit.
    - Commit messages should provide a clear and concise explanation of the solution's rationale.
    - If the specific commit closes some specific github issue, include the issue URL in the commit message.
    - If possible, sign your commits with your GPG key.
    - Writing style: [chris.beams.io/posts/git-commit](https://chris.beams.io/posts/git-commit/)
    - Structure: [conventionalcommits.org](https://www.conventionalcommits.org/)

5. **Submit a Pull Request**

    Once you're satisfied with your changes, submit a pull request to this repository. Provide a clear and concise description of the changes you've made. If your pull request addresses an existing issue, reference the issue number in the description. Every PR must be opened against the `main` branch.

6. **Review and Iterate**

7. **Merge and Close**

    Once your pull request has been approved and all discussions have been resolved, a project maintainer will merge your changes into the `main` branch. Your contribution will then be officially part of the project. The pull request will be closed, marking the completion of your contribution.

### Your First Code Contribution
>In order to contribute, a basic learning about git and github is needed. If you're not familiar with them, have a look at https://docs.github.com/en/get-started/start-your-journey/git-and-github-learning-resources to dig into and learn how to use them.

Unsure where to begin contributing? Every ❌ in the [Suite Coverage](COVERAGE.md#suite-coverage) table is a scenario that is already specified in the catalog and still waiting for an implementation.

Another way to better understand where to focus your contribution is by looking at our project board: https://github.com/orgs/stratum-mining/projects/25
