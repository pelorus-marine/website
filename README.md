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

## Release: GHCR + Artifact Registry + Cloud Run

A tag matching `v*` or a manual **Actions → Release → Run workflow** runs [`.github/workflows/release.yml`](.github/workflows/release.yml): build once, push to **GHCR** and **Artifact Registry**, deploy **Cloud Run** from Artifact Registry.

Cloud Run only pulls from `gcr.io`, `REGION-docker.pkg.dev`, or `docker.io` — not `ghcr.io` unless you add an [Artifact Registry remote repo](https://cloud.google.com/artifact-registry/docs/repositories/remote-repo). This workflow pushes the same image to GHCR and Artifact Registry.

### One-time: GCP from scratch (`europe-west1`)

Use [`.github/workflows/release.yml`](.github/workflows/release.yml) after this. From a clean project (billing on):

```bash
gcloud config set project YOUR_PROJECT_ID
PROJECT_ID=YOUR_PROJECT_ID \
GITHUB_REPO=your-org/your-repo \
./scripts/setup-gcp.sh
```

This enables APIs, creates a **Docker** Artifact Registry repo (default id `website` in **`europe-west1`**), Workload Identity Federation for GitHub Actions, and a deployer service account. It does **not** create a Cloud Run service — the **Release** workflow (on tag `v*`) or a manual `gcloud run deploy` does that on first push. Override region only if you need another: `GCP_REGION=… ./scripts/setup-gcp.sh`.

Image URL shape:

`europe-west1-docker.pkg.dev/PROJECT_ID/website/website:TAG`

(`website` twice = Artifact Registry **repository id** vs image name in the workflow.)

### GitHub configuration

Settings → **Secrets and variables** → **Actions** (not Dependabot/Codespaces). Organization variables must be allowed for this repository.

The job uses GitHub **Environment** **`gcp`** by default (create it under Settings → Environments, or set repository variable **`GCP_ACTIONS_ENVIRONMENT`** to another name). Put deploy **variables/secrets and WIF secrets in the same scope** (repository or that environment); environment-only values are not visible without a matching `environment:` on the job.

**Variables** (preferred) or **Secrets** with the same names — workflow accepts either:

| Name | Example |
|------|---------|
| `GCP_PROJECT_ID` | your project id |
| `GCP_REGION` | `europe-west1` (match Artifact Registry + Cloud Run; avoid CRLF pastes) |
| `GCP_ARTIFACT_REPOSITORY` | `website` (Artifact Registry repository id) |
| `CLOUD_RUN_SERVICE` | e.g. `pelorus-website` (must match workflow deploy name) |

**Secrets** (always):

| Name | Value |
|------|--------|
| `GCP_WORKLOAD_IDENTITY_PROVIDER` | Full provider resource name (see below) |
| `GCP_SERVICE_ACCOUNT` | Deployer service account email |

### Workload Identity Federation in one minute

Normally you’d put a GCP **JSON key** in GitHub so Actions can call `gcloud`. That key is a secret you must rotate and protect.

**Workload Identity Federation (WIF)** avoids keys: GitHub proves who it is with a short-lived OIDC token; Google trusts GitHub and lets that workflow **act as** one specific **Google Cloud service account** — a “robot user” in your project (email ends in `@…iam.gserviceaccount.com`). That robot account is what people mean by the **“WIF service account”** here: it’s just the GCP service account you bind to the GitHub pool/provider.

**Scripted setup** is **`./scripts/setup-gcp.sh`** (see **One-time: GCP from scratch** above). `GITHUB_REPO` must be the full **`owner/repo`** OIDC claim (e.g. `pelorus-marine/website`). The script prints WIF secrets and the GitHub Variables to set; optional env is documented in the script header.

**Manual setup:** follow [Authenticate to Google Cloud from GitHub Actions](https://cloud.google.com/iam/docs/workload-identity-federation-with-deployment-pipelines) or the [google-github-actions/auth README](https://github.com/google-github-actions/auth#preferred-direct-workload-identity-federation). In outline: pool → GitHub provider → service account → IAM binding so only your repo can impersonate that SA → paste the provider resource name and SA email into the secrets above.

No JSON key is stored in GitHub.

The deploy service account needs at least:

- **Artifact Registry**: `roles/artifactregistry.writer` on the repository (or project), to push images.
- **Cloud Run**: e.g. **Cloud Run Admin** and **Service Account User** on the runtime service account, to deploy and invoke the service.

## Licensing

Source code in this repository (for example Rust sources under `src/`, templates under `templates/`, and HTML/CSS authored for this site) is licensed under the MIT License **or** the Apache License, Version 2.0, at your option. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

The Pelorus word mark, Pelorus Marine name and branding, and image assets under `static/` (including `pelorus-favicon-32.png`, `pelorus-icon-200.png`, and `pelorus-icon-500.png`) are proprietary to Pelorus Marine. All rights reserved. Those assets are not licensed under MIT or Apache-2.0.

Third-party CSS bundled with the site (Bootstrap under `static/vendor/bootstrap-5.3.3/`) remains under its license (Bootstrap: MIT).

The display face **Operation Napalm** (Regular), file `static/fonts/operation-napalm-regular.woff2`, comes from the [fonts-cc0](https://github.com/ggbotnet/fonts-cc0) collection: folder *Operation Napalm* → *Web Open Font Format (.woff)* → `OperationNapalm-Regular.woff2`. That collection is under [**CC0 1.0 Universal**](https://creativecommons.org/publicdomain/zero/1.0/legalcode) (public domain dedication). No attribution is required; the font is provided as-is with no warranty.