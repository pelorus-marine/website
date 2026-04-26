# Pelorus website

Minimal site for [Pelorus Marine](https://github.com/pelorus-marine): [warp](https://github.com/seanmonstar/warp), [Askama](https://github.com/djc/askama) templates, and Bootstrap 5.3.3 (vendored under `static/vendor/` so the page works without a CDN).

## Run locally

Requires a recent Rust toolchain.

```bash
cargo run
```

The server listens on `http://0.0.0.0:8080`. Routes: `/` and `/pelorus` (same page), plus `/static/*` for bundled assets.

## Test

```bash
cargo test
```

Unit tests live in `src/lib.rs`; integration tests in `tests/http.rs`. GitHub Actions (`.github/workflows/ci.yml`) runs `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo test` on pushes and pull requests to `main` or `master`.

## Licensing

Source code in this repository (for example Rust sources under `src/`, templates under `templates/`, and HTML/CSS authored for this site) is licensed under the MIT License **or** the Apache License, Version 2.0, at your option. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

The Pelorus word mark, Pelorus Marine name and branding, and image assets under `static/` (including `pelorus-favicon-32.png`, `pelorus-icon-200.png`, and `pelorus-icon-500.png`) are proprietary to Pelorus Marine. All rights reserved. Those assets are not licensed under MIT or Apache-2.0.

Third-party CSS bundled with the site (Bootstrap under `static/vendor/bootstrap-5.3.3/`) remains under its license (Bootstrap: MIT).

The display face **Operation Napalm** (Regular), file `static/fonts/operation-napalm-regular.woff2`, comes from the [fonts-cc0](https://github.com/ggbotnet/fonts-cc0) collection: folder *Operation Napalm* → *Web Open Font Format (.woff)* → `OperationNapalm-Regular.woff2`. That collection is under [**CC0 1.0 Universal**](https://creativecommons.org/publicdomain/zero/1.0/legalcode) (public domain dedication). No attribution is required; the font is provided as-is with no warranty.
