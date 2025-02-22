FROM rustlang/rust:nightly-slim AS backend

WORKDIR /back

COPY Cargo.toml .
RUN mkdir src && touch src/lib.rs
RUN cargo build --release

COPY src src
RUN touch src/lib.rs
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt update && apt install tini

COPY --from=backend /back/target/release/me-tube /usr/local/bin/me-tube

WORKDIR /app

ENTRYPOINT ["tini", "--", "/usr/local/bin/me-tube"]
