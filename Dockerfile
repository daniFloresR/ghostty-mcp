# Stage 1: Build static binary
FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY data/ data/
COPY scripts/ scripts/
RUN cargo build --release

# Stage 2: Minimal runtime
FROM scratch
LABEL org.opencontainers.image.source="https://github.com/daniFloresR/ghostty-mcp"
LABEL org.opencontainers.image.description="MCP server for Ghostty terminal configuration"
LABEL org.opencontainers.image.licenses="MIT"
LABEL io.modelcontextprotocol.server.name="io.github.danifloresr/ghostty-mcp"
COPY --from=builder /app/target/release/ghostty-mcp /ghostty-mcp
ENTRYPOINT ["/ghostty-mcp"]
