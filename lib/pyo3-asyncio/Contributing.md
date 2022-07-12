# Contributing

Thank you for your interest in contributing to PyO3! All are welcome - please consider reading our [Code of Conduct](Code-of-Conduct.md) to keep our community positive and inclusive.

If you are searching for ideas how to contribute, please read the "Getting started contributing" section. Once you've found an issue to contribute to, you may find the section "Writing pull requests" helpful.

## Getting started contributing

Please join in with any part of PyO3 which interests you. We use Github issues to record all bugs and ideas. Feel free to request an issue to be assigned to you if you want to work on it.

The following sections also contain specific ideas on where to start contributing to PyO3.

### Help users identify bugs

The [PyO3 Gitter channel](https://gitter.im/PyO3/Lobby) is very active with users who are new to PyO3, and often completely new to Rust. Helping them debug is a great way to get experience with the PyO3 codebase.

Helping others often reveals bugs, documentation weaknesses, and missing APIs. It's a good idea to open Github issues for these immediately so the resolution can be designed and implemented!

### Review pull requests

Everybody is welcome to submit comments on open PRs. Please help ensure new PyO3 APIs are safe, performant, tidy, and easy to use!

## Writing pull requests

Here are a few things to note when you are writing PRs.

### Continuous Integration

The PyO3 Asyncio repo uses Github Actions. PRs are blocked from merging if CI is not successful.

Formatting, linting and tests are checked for all Rust and Python code. In addition, all warnings in Rust code are disallowed (using `RUSTFLAGS="-D warnings"`).

Tests run with all supported Python versions with the latest stable Rust compiler, as well as for Python 3.9 with the minimum supported Rust version.

### Minimum supported Rust version

PyO3 aims to make use of up-to-date Rust language features to keep the implementation as efficient as possible.

However, there will always be support for at least the last few Rust compiler versions, so that users have time to update.

If your PR needs to bump the minimum supported Rust version, this is acceptable with the following conditions:
- Any changes which require a more recent version than what is [currently available on stable Red Hat Enterprise Linux](https://access.redhat.com/documentation/en-us/red_hat_developer_tools/1/) will be postponed. (This is to allow package managers to update support for newer `rustc` versions; RHEL was arbitrarily picked because their update policy is clear.)
- You might be asked to do extra work to tidy up other parts of the PyO3 codebase which can use the compiler version bump :)


## Git Hooks (Recommended)

Using the project's githooks are recommended to prevent CI from failing for trivial reasons such as build checks or integration tests failing. Enabling the hooks should run `cargo check --all-targets` for every commit and `cargo test` for every push.

```
git config core.hookspath .githooks
```