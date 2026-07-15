FROM rust:1.93-bookworm AS rust-build
RUN rustup target add wasm32-unknown-unknown
WORKDIR /src
COPY . .
RUN cargo build --locked --release -p iwm-api \
    && cargo build --locked --release -p iwm-runtime-web --target wasm32-unknown-unknown

FROM node:22-bookworm-slim AS web-build
WORKDIR /src/runtime
COPY runtime/package.json runtime/package-lock.json ./
RUN npm ci
COPY runtime/ ./
COPY --from=rust-build /src/target/wasm32-unknown-unknown/release/iwm_runtime_web.wasm ./public/wasm/iwm_runtime_web.wasm
RUN npm run build

FROM debian:bookworm-slim
ARG VERSION=0.2.0-beta.2
ARG VCS_REF=unknown
LABEL org.opencontainers.image.title="IWanna GM8 Web Engine Beta" \
      org.opencontainers.image.version="$VERSION" \
      org.opencontainers.image.revision="$VCS_REF" \
      org.opencontainers.image.source="https://github.com/horizonzzzz/iwanna-gm8-web-engine"
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=rust-build /src/target/release/iwm-api /usr/local/bin/iwm-api
COPY --from=web-build /src/runtime/dist/ /app/static/
COPY LICENSE NOTICE.md /usr/share/doc/iwm-api/
COPY vendor/OpenGMK/LICENCE.md /usr/share/doc/iwm-api/GPL-2.0-only.txt
RUN mkdir -p /data \
    && chown 10001:0 /data \
    && chmod g=u /data
USER 10001
ENV IWM_BIND=0.0.0.0:3000 \
    IWM_DATA_DIR=/data \
    IWM_STATIC_DIR=/app/static
VOLUME ["/data"]
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=15s --retries=3 \
  CMD curl --fail --silent http://127.0.0.1:3000/healthz || exit 1
CMD ["iwm-api"]
