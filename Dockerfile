# syntax=docker/dockerfile:1.6
# Runtime-only image. Build context must be the directory produced by
# `scripts/prepare-image-context.sh` (contains `website` binary + `static/`).
# Distroless `base` is smaller but lacks libgcc_s.so.1, which this glibc-linked
# Rust binary needs; `cc` is the smallest stock distroless image that fits.
FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app

COPY --chmod=755 website /app/website
COPY --chown=nonroot:nonroot static ./static

USER nonroot:nonroot

ENV PORT=8080
EXPOSE 8080

ENTRYPOINT ["/app/website"]
