#!/usr/bin/env bash
# One-shot GCP bootstrap for this repo: APIs, Artifact Registry (Docker), WIF for GitHub Actions,
# deployer service account (push AR + deploy Cloud Run). Region defaults to europe-west1 (Cloud Run
# custom domain mapping is supported there). No JSON keys.
#
# Prerequisites: gcloud installed; `gcloud auth login`; billing on the project.
#
# Usage:
#   PROJECT_ID=your-gcp-project \
#   GITHUB_REPO=org-or-user/repo-name \
#   ./scripts/setup-gcp.sh
#
# Optional env:
#   GCP_REGION           default: europe-west1
#   GCP_ARTIFACT_REPOSITORY   default: website  (Docker repo id in Artifact Registry)
#   CLOUD_RUN_SERVICE    default: pelorus-website  (printed for GitHub variable; first deploy creates it)
#   POOL_ID              default: github-actions
#   PROVIDER_ID          default: github
#   SA_ID                default: github-actions-deploy
#   CLOUD_RUN_RUNTIME_SA  default: PROJECT_NUMBER-compute@developer.gserviceaccount.com
#
set -euo pipefail

die() {
  echo "error: $*" >&2
  exit 1
}

[[ -n "${PROJECT_ID:-}" ]] || die "set PROJECT_ID (GCP project id)"
[[ -n "${GITHUB_REPO:-}" ]] || die "set GITHUB_REPO (e.g. pelorus-marine/website)"
[[ "${GITHUB_REPO}" == *"/"* ]] || die "GITHUB_REPO must be owner/repo, not just the repo slug (got: ${GITHUB_REPO})"

REGION="${GCP_REGION:-europe-west1}"
ARTIFACT_REPO="${GCP_ARTIFACT_REPOSITORY:-website}"
CLOUD_RUN_HINT="${CLOUD_RUN_SERVICE:-pelorus-website}"
POOL_ID="${POOL_ID:-github-actions}"
PROVIDER_ID="${PROVIDER_ID:-github}"
SA_ID="${SA_ID:-github-actions-deploy}"
SA_EMAIL="${SA_ID}@${PROJECT_ID}.iam.gserviceaccount.com"

PROJECT_NUMBER="$(gcloud projects describe "${PROJECT_ID}" --format='value(projectNumber)')"
[[ -n "${PROJECT_NUMBER}" ]] || die "could not resolve project number for ${PROJECT_ID}"

RUNTIME_SA="${CLOUD_RUN_RUNTIME_SA:-${PROJECT_NUMBER}-compute@developer.gserviceaccount.com}"

echo "==> Enabling APIs"
gcloud services enable \
  iamcredentials.googleapis.com \
  sts.googleapis.com \
  iam.googleapis.com \
  artifactregistry.googleapis.com \
  run.googleapis.com \
  cloudresourcemanager.googleapis.com \
  --project="${PROJECT_ID}"

# New projects: IAM can lag a few seconds after enable + first SA creation.
echo "==> Waiting for IAM/API propagation (15s)"
sleep 15

echo "==> Artifact Registry Docker repository: ${ARTIFACT_REPO} (${REGION})"
if ! gcloud artifacts repositories describe "${ARTIFACT_REPO}" \
  --location="${REGION}" \
  --project="${PROJECT_ID}" &>/dev/null; then
  gcloud artifacts repositories create "${ARTIFACT_REPO}" \
    --repository-format=docker \
    --location="${REGION}" \
    --project="${PROJECT_ID}" \
    --description="Website container images"
fi

echo "==> Workload Identity Pool: ${POOL_ID}"
if ! gcloud iam workload-identity-pools describe "${POOL_ID}" \
  --location=global \
  --project="${PROJECT_ID}" &>/dev/null; then
  gcloud iam workload-identity-pools create "${POOL_ID}" \
    --project="${PROJECT_ID}" \
    --location=global \
    --display-name="GitHub Actions (${POOL_ID})"
fi

echo "==> OIDC provider: ${PROVIDER_ID} (GitHub)"
if ! gcloud iam workload-identity-pools providers describe "${PROVIDER_ID}" \
  --location=global \
  --workload-identity-pool="${POOL_ID}" \
  --project="${PROJECT_ID}" &>/dev/null; then
  gcloud iam workload-identity-pools providers create-oidc "${PROVIDER_ID}" \
    --project="${PROJECT_ID}" \
    --location=global \
    --workload-identity-pool="${POOL_ID}" \
    --display-name="GitHub Actions OIDC" \
    --issuer-uri="https://token.actions.githubusercontent.com" \
    --attribute-mapping="google.subject=assertion.sub,attribute.actor=assertion.actor,attribute.repository=assertion.repository,attribute.repository_owner=assertion.repository_owner" \
    --attribute-condition="assertion.repository=='${GITHUB_REPO}'"
fi

echo "==> Service account: ${SA_EMAIL}"
if ! gcloud iam service-accounts describe "${SA_EMAIL}" \
  --project="${PROJECT_ID}" &>/dev/null; then
  gcloud iam service-accounts create "${SA_ID}" \
    --project="${PROJECT_ID}" \
    --display-name="GitHub Actions deploy (WIF)"
fi

# Project IAM rejects bindings until the SA is visible everywhere (eventual consistency).
echo "==> Waiting for service account ${SA_EMAIL} to be bindable"
for _ in $(seq 1 30); do
  if gcloud iam service-accounts describe "${SA_EMAIL}" \
    --project="${PROJECT_ID}" &>/dev/null; then
    break
  fi
  sleep 2
done
gcloud iam service-accounts describe "${SA_EMAIL}" \
  --project="${PROJECT_ID}" &>/dev/null \
  || die "service account ${SA_EMAIL} still not visible — check PROJECT_ID and IAM permissions"

echo "==> Project roles for ${SA_EMAIL}"
for role in roles/artifactregistry.writer roles/run.admin; do
  ok=0
  for attempt in 1 2 3 4 5 6; do
    if gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
      --member="serviceAccount:${SA_EMAIL}" \
      --role="${role}" \
      --quiet; then
      ok=1
      break
    fi
    echo "warn: ${role} bind failed (attempt ${attempt}/6), retrying in 10s..." >&2
    sleep 10
  done
  [[ "${ok}" -eq 1 ]] \
    || die "failed to bind ${role} to ${SA_EMAIL}. If you see 'condition' errors, run: gcloud alpha iam policies lint-condition --project=${PROJECT_ID}"
done

echo "==> Let deployer act as Cloud Run runtime SA: ${RUNTIME_SA}"
gcloud iam service-accounts add-iam-policy-binding "${RUNTIME_SA}" \
  --project="${PROJECT_ID}" \
  --member="serviceAccount:${SA_EMAIL}" \
  --role="roles/iam.serviceAccountUser" \
  --quiet

PRINCIPAL="principalSet://iam.googleapis.com/projects/${PROJECT_NUMBER}/locations/global/workloadIdentityPools/${POOL_ID}/attribute.repository/${GITHUB_REPO}"

echo "==> Allow GitHub repo ${GITHUB_REPO} to impersonate ${SA_EMAIL}"
gcloud iam service-accounts add-iam-policy-binding "${SA_EMAIL}" \
  --project="${PROJECT_ID}" \
  --member="${PRINCIPAL}" \
  --role="roles/iam.workloadIdentityUser" \
  --quiet

WIF_PROVIDER="projects/${PROJECT_NUMBER}/locations/global/workloadIdentityPools/${POOL_ID}/providers/${PROVIDER_ID}"
AR_HOST="${REGION}-docker.pkg.dev"

echo ""
echo "Done. Set GitHub Actions **Secrets** (same scope: repository or Environment gcp):"
echo ""
echo "  GCP_WORKLOAD_IDENTITY_PROVIDER=${WIF_PROVIDER}"
echo "  GCP_SERVICE_ACCOUNT=${SA_EMAIL}"
echo ""
echo "Set GitHub Actions **Variables** (or Secrets):"
echo ""
echo "  GCP_PROJECT_ID=${PROJECT_ID}"
echo "  GCP_REGION=${REGION}"
echo "  GCP_ARTIFACT_REPOSITORY=${ARTIFACT_REPO}"
echo "  CLOUD_RUN_SERVICE=${CLOUD_RUN_HINT}"
echo ""
echo "Image URL shape: ${AR_HOST}/${PROJECT_ID}/${ARTIFACT_REPO}/website:TAG"
echo "Cloud Run in ${REGION} supports console **Domain mappings** (preview); otherwise use a global HTTPS LB."
echo ""
