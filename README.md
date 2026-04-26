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

## Release: GHCR + Artifact Registry + Cloud Run (tags only)

Pushing a tag matching `v*` (e.g. `v1.0.0`) runs [`.github/workflows/release.yml`](.github/workflows/release.yml): build once, push to **GitHub Container Registry** and **Google Artifact Registry**, then deploy **Cloud Run from Artifact Registry**.

Cloud Run only accepts images from `gcr.io`, `REGION-docker.pkg.dev` (Artifact Registry), or `docker.io` — **not** `ghcr.io` unless you add an [Artifact Registry remote repository](https://cloud.google.com/artifact-registry/docs/repositories/remote-repo). This repo pushes the same image to both GHCR (for you / GitHub) and Artifact Registry (for deploy).

### One-time: Artifact Registry Docker repository

Pick an id (example: `website`). Same region as Cloud Run is typical:

```bash
gcloud artifacts repositories create website \
  --repository-format=docker \
  --location=europe-southwest1 \
  --description="Website images"
```

Use that id as **`GCP_ARTIFACT_REPOSITORY`** below. Images will be:

`europe-southwest1-docker.pkg.dev/PROJECT_ID/website/website:TAG`

### GitHub configuration

**Repository variables** (Settings → Secrets and variables → Actions → **Variables** — preferred for non-sensitive values):

| Variable | Example | Purpose |
|----------|---------|---------|
| `GCP_PROJECT_ID` | `seven-seas-494519` | GCP project |
| `GCP_REGION` | `europe-southwest1` | Cloud Run + Artifact Registry location |
| `GCP_ARTIFACT_REPOSITORY` | `website` | Artifact Registry **repository id** (Docker) |
| `CLOUD_RUN_SERVICE` | `sevenseas-website` | Cloud Run service name |
| `GCP_ACTIONS_ENVIRONMENT` | *(omit or `production`)* | GitHub **Environment** name for the release job; default in workflow is `gcp` |

If you already added these four under **Secrets** instead, the release workflow still picks them up (same names). They must live under **Actions** secrets/variables, not the Dependabot or Codespaces tabs. For **organization** variables, each name must be allowed for this repository.

**GitHub Environment (common gotcha):** The release job uses `environment: gcp` by default (or whatever you set in repository variable **`GCP_ACTIONS_ENVIRONMENT`**). Values stored **only** under **Settings → Environments → _name_ → Environment secrets/variables** are invisible unless the workflow sets a matching `environment`. Either:

- Create an environment named **`gcp`** (no protection rules needed) and add the same names there, **or**
- Keep using **repository**-level Variables/Secrets (they still work; the job may reference an empty environment you create once).

Put **`GCP_WORKLOAD_IDENTITY_PROVIDER`** and **`GCP_SERVICE_ACCOUNT`** in the **same** place as the four deploy keys (repository or that environment), not split across scopes.

**Manual test:** **Actions** → **Release** → **Run workflow** — optional inputs override Variables/Secrets for that run. Open the job summary to see a **length** table (not values) for debugging.

**Repository secrets**:

| Secret | What to paste |
|--------|----------------|
| `GCP_WORKLOAD_IDENTITY_PROVIDER` | Full **provider resource name** (see below) |
| `GCP_SERVICE_ACCOUNT` | **Service account email** you create in GCP (see below) |

### Workload Identity Federation in one minute

Normally you’d put a GCP **JSON key** in GitHub so Actions can call `gcloud`. That key is a secret you must rotate and protect.

**Workload Identity Federation (WIF)** avoids keys: GitHub proves who it is with a short-lived OIDC token; Google trusts GitHub and lets that workflow **act as** one specific **Google Cloud service account** — a “robot user” in your project (email ends in `@…iam.gserviceaccount.com`). That robot account is what people mean by the **“WIF service account”** here: it’s just the GCP service account you bind to the GitHub pool/provider.

**Scripted setup** (after `gcloud auth login` and `gcloud config set project …`):

```bash
PROJECT_ID=your-gcp-project \
GITHUB_REPO=your-org/your-repo \
./scripts/setup-gcp-wif-github-actions.sh
```

`GITHUB_REPO` must be the full **`owner/repo`** string GitHub puts in OIDC (for example `pelorus-marine/website`), not just `website`.

The script enables APIs, creates a workload identity pool + GitHub OIDC provider (restricted to that repo), creates a deployer service account, grants **Artifact Registry Writer** and **Cloud Run Admin**, lets the deployer use the default **Compute** service account as the Cloud Run runtime identity, prints **`GCP_WORKLOAD_IDENTITY_PROVIDER`** and **`GCP_SERVICE_ACCOUNT`** for GitHub secrets. Optional env: `POOL_ID`, `PROVIDER_ID`, `SA_ID`, `CLOUD_RUN_RUNTIME_SA` (see script header).

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