FROM rust:1.60.0-slim
COPY ./ ./
RUN cargo build --release
ENTRYPOINT ["./target/release/sulphire", "default"]