FROM rustlang/rust:nightly-slim AS backend

WORKDIR /back

COPY Cargo.toml .
RUN mkdir -p src/bin && \
    touch src/lib.rs && \
    touch src/bin/admin.rs && \
    echo "fn main() { println!(\"if you see this, the build broke\") }" | tee src/bin/main.rs src/bin/admin.rs > /dev/null
RUN cargo build --release

COPY src src
RUN touch src/lib.rs
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt update && apt install tini

COPY --from=backend /back/target/release/me-tube /usr/local/bin/me-tube
COPY --from=backend /back/target/release/me-tube-admin /usr/local/bin/me-tube-admin

WORKDIR /app

ENTRYPOINT ["tini", "--", "/usr/local/bin/me-tube"]
