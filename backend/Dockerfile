FROM rustlang/rust:nightly-slim AS backend

WORKDIR /back

COPY Cargo.toml Cargo.lock .
RUN mkdir src && echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs
RUN cargo build --release

COPY src src
RUN touch src/main.rs
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt update && apt install tini

COPY --from=backend /back/target/release/me-tube /usr/local/bin/me-tube

WORKDIR /app

ENTRYPOINT ["tini", "--", "/usr/local/bin/me-tube"]
