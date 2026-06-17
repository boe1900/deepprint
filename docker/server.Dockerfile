# syntax=docker/dockerfile:1.7

FROM rust:1-bookworm AS builder

WORKDIR /work

ARG TARGETARCH

COPY apps/server/Cargo.toml apps/server/Cargo.lock ./apps/server/
COPY apps/server/src ./apps/server/src

RUN --mount=type=cache,id=server-cargo-registry-${TARGETARCH},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=server-cargo-git-${TARGETARCH},target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=server-cargo-target-${TARGETARCH},target=/work/apps/server/target,sharing=locked \
    cargo build --manifest-path apps/server/Cargo.toml --release \
    && install -Dm755 /work/apps/server/target/release/deepprint-server /work/deepprint-server

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        fontconfig \
        fonts-noto-cjk \
        fonts-noto-core \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /work/deepprint-server /usr/local/bin/deepprint-server
COPY apps/server/assets/fonts /opt/deepprint/default-fonts

ENV DEEPPRINT_AGENT_BIND=0.0.0.0
ENV DEEPPRINT_AGENT_PORT=17801
ENV DEEPPRINT_DATABASE_URL=sqlite:///data/deepprint.db
ENV DEEPPRINT_CUPS_BASE_URL=http://cups:631/
ENV DEEPPRINT_RENDER_ENGINE=typst
ENV DEEPPRINT_TYPST_LOCAL_PACKAGES_ROOT=/data/typst/packages
ENV DEEPPRINT_TYPST_PREVIEW_CACHE_ROOT=/data/cache/typst
ENV DEEPPRINT_TYPST_FONTS_ROOT=/data/typst/fonts
ENV DEEPPRINT_TYPST_DEFAULT_FONTS_ROOT=/opt/deepprint/default-fonts

VOLUME ["/data"]
EXPOSE 17801

CMD ["deepprint-server"]
