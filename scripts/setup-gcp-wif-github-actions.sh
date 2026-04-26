#!/usr/bin/env bash
# Provision Workload Identity Federation for GitHub Actions + a deployer service account
# (Artifact Registry push + Cloud Run deploy). No JSON keys.
#
# Prerequisites: gcloud installed; logged in; billing enabled on the project.
#
# Usage:
#   PROJECT_ID=your-gcp-project \
#   GITHUB_REPO=org-or-user/repo-name \
#   ./scripts/setup-gcp-wif-github-actions.sh
#
# Optional env (defaults shown):
#   POOL_ID=github-actions
#   PROVIDER_ID=github
#   SA_ID=github-actions-deploy
#   CLOUD_RUN_RUNTIME_SA=   # default: PROJECT_NUMBER-compute@developer.gserviceaccount.com
#
set -euo pipefail

die() {
  echo "error: $*" >&2
  exit 1
}

[[ -n "${PROJECT_ID:-}" ]] || die "set PROJECT_ID (GCP project id)"
[[ -n "${GITHUB_REPO:-}" ]] || die "set GITHUB_REPO (e.g. pelorus-marine/website)"
[[ "${GITHUB_REPO}" == *"/"* ]] || die "GITHUB_REPO must be owner/repo (GitHub's repository claim), not just the repo name — e.g. pelorus-marine/website (got: ${GITHUB_REPO})"

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
  --project="${PROJECT_ID}"

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

echo "==> Project roles for ${SA_EMAIL}"
for role in roles/artifactregistry.writer roles/run.admin; do
  gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="${role}" \
    --quiet
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

echo ""
echo "Done. Add these as GitHub repository secrets (Settings → Secrets and variables → Actions):"
echo ""
echo "  Name: GCP_WORKLOAD_IDENTITY_PROVIDER"
echo "  Value: ${WIF_PROVIDER}"
echo ""
echo "  Name: GCP_SERVICE_ACCOUNT"
echo "  Value: ${SA_EMAIL}"
echo ""
echo "Repository Actions variables still required: GCP_PROJECT_ID, GCP_REGION, GCP_ARTIFACT_REPOSITORY, CLOUD_RUN_SERVICE."
echo "Create the Artifact Registry Docker repo first if you have not (see README)."
