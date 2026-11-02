# Third-Party Notices — Template

This file is a template, not a completed attribution inventory. Before
distribution, list every bundled third-party component, version, copyright
holder, license, and required notice. Include native libraries, models, and
frontend production dependencies as well as Rust crates.

For each component, add:

```text
Component:
Version or immutable revision:
Source URL:
Copyright holder:
License identifier:
License text or required notice:
Shipped files / purpose:
Redistribution review:
```

Task 45 adds `icu_casemap` 2.4.0 and `icu_normalizer` 2.4.0 to `outflow-core`,
along with resolved ICU4X and Unicode-data dependencies. Cargo metadata reports
Unicode-3.0 for ICU4X and Unicode-derived crates; the full locked license list
and required notice-review scope are recorded in
`evidence/rust-vocabulary-icu4x-task-45.md`. This template is still not a
completed attribution inventory or a packaged notice bundle. The frontend,
build tools, native libraries, and model assets require their own review.
