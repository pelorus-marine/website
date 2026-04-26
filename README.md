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

Unit tests live in `src/lib.rs`; integration tests in `tests/http.rs`. GitHub Actions (`.github/workflows/ci.yml`) runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, and a **Docker image build** (no push) on pushes and pull requests to `main` or `master`.

## Container image

The **Rust release binary is built on the host or in CI**; the Dockerfile only packages that binary plus `static/` into a Distroless runtime (no compile inside Docker).

```bash
cargo build --release --locked
./scripts/prepare-image-context.sh   # writes build/image/{website,static/}
docker build -f Dockerfile -t pelorus-website:local build/image
docker run --rm -p 8080:8080 pelorus-website:local
```

`build/` is gitignored. The binary listens on **`0.0.0.0`** and **`PORT`** (default `8080`) for [Cloud Run](https://cloud.google.com/run/docs/container-contract).

## Release: GHCR + Cloud Run (tags only)

Pushing a tag matching `v*` (e.g. `v1.0.0`) runs [`.github/workflows/release.yml`](.github/workflows/release.yml): build and push to **GitHub Container Registry** (`ghcr.io/<owner>/<repo>`), then deploy that digest to **Google Cloud Run**.

### GitHub configuration

**Repository variables** (Settings → Secrets and variables → Actions → Variables):

| Variable | Example | Purpose |
|----------|---------|---------|
| `GCP_PROJECT_ID` | `my-project-123` | GCP project |
| `GCP_REGION` | `europe-southwest1` | Cloud Run region |
| `CLOUD_RUN_SERVICE` | `pelorus-website` | Service name |

**Repository secrets**:

| Secret | Purpose |
|--------|---------|
| `GCP_WORKLOAD_IDENTITY_PROVIDER` | Full WIF provider resource name |
| `GCP_SERVICE_ACCOUNT` | Deployer service account email |

Use [Workload Identity Federation](https://github.com/google-github-actions/auth#preferred-direct-workload-identity-federation) so no long-lived JSON keys are stored in GitHub. The deploy service account needs permission to create/update Cloud Run services (for example roles **Cloud Run Admin** and **Service Account User** on the runtime service account).

### GHCR and Cloud Run

Cloud Run pulls the image from `ghcr.io`. The simplest setup is a **public** GHCR package (or public image visibility) so the platform can pull without extra registry credentials. If the package must stay **private**, use Google’s docs for [private container images](https://cloud.google.com/run/docs/deploying#private-registry) (for example mirroring to Artifact Registry or configuring pull access).

## Licensing

Source code in this repository (for example Rust sources under `src/`, templates under `templates/`, and HTML/CSS authored for this site) is licensed under the MIT License **or** the Apache License, Version 2.0, at your option. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

The Pelorus word mark, Pelorus Marine name and branding, and image assets under `static/` (including `pelorus-favicon-32.png`, `pelorus-icon-200.png`, and `pelorus-icon-500.png`) are proprietary to Pelorus Marine. All rights reserved. Those assets are not licensed under MIT or Apache-2.0.

Third-party CSS bundled with the site (Bootstrap under `static/vendor/bootstrap-5.3.3/`) remains under its license (Bootstrap: MIT).

The display face **Operation Napalm** (Regular), file `static/fonts/operation-napalm-regular.woff2`, comes from the [fonts-cc0](https://github.com/ggbotnet/fonts-cc0) collection: folder *Operation Napalm* → *Web Open Font Format (.woff)* → `OperationNapalm-Regular.woff2`. That collection is under [**CC0 1.0 Universal**](https://creativecommons.org/publicdomain/zero/1.0/legalcode) (public domain dedication). No attribution is required; the font is provided as-is with no warranty.
