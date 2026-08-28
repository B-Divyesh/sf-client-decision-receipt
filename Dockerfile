FROM node:22-bookworm-slim AS web
WORKDIR /src
COPY package.json package-lock.json vite.config.ts ./
COPY frontend ./frontend
RUN npm ci && npm run build

FROM rust:1.98-bookworm AS server
ARG BUILD_SHA=dev
WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs ./
COPY migrations ./migrations
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home app \
    && mkdir -p /app/dist /data && chown -R app:app /app /data
WORKDIR /app
COPY --from=server /src/target/release/client-decision-receipt /app/server
COPY --from=web /src/dist /app/dist
USER 10001
EXPOSE 8080
ENV PORT=8080
CMD ["/app/server"]
