# Contributing to Ollama Cluster

Thank you for considering a contribution. Ollama Cluster is source-available under [LICENSE.md](LICENSE.md). We welcome bug fixes, improvements, documentation, and tests from everyone — you do not need a Commercial License to contribute.

## Developer Certificate of Origin (DCO)

By contributing to this project, you agree to the [Developer Certificate of Origin (DCO)](https://developercertificate.org/):

```
Developer Certificate of Origin
Version 1.1

Copyright (C) 2004, 2006 The Linux Foundation and its contributors.

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.

Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the license of the project.

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part
    by me, under the same license (unless I am permitted to submit
    under a different license).

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including all
    personal information I submit with it) is maintained indefinitely
    and may be redistributed consistent with this project or the
    license(s) involved.
```

Sign your commits with:

```bash
git commit -s -m "Your commit message"
```

The `-s` flag adds a `Signed-off-by:` line to your commit message.

## How to contribute

1. Fork the repository and create a branch from `main`
2. Make your changes with tests where appropriate
3. Run `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`
4. Commit with `-s` (sign-off)
5. Open a pull request describing what changed and why

## Code guidelines

- Match existing code style and conventions in the crate you are editing
- Keep changes focused — one logical change per pull request where possible
- Add or update tests for behaviour changes
- Update documentation when user-facing behaviour changes

## Licensing of contributions

By submitting a pull request, you agree that your contribution may be distributed under the project's [LICENSE.md](LICENSE.md) and that the copyright holder may also offer the software under separate commercial license terms to third parties.

## Questions

Open a [GitHub Issue](https://github.com/levi-putna/ollama-cluster/issues) for questions about contributing or licensing.
