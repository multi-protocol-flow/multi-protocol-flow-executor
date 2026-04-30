# ============================================================
# Stage 1: Builder — 编译 protocol-test-server
# ============================================================
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/protocol-test-server

# 依赖缓存层：先复制 Cargo 文件，用 dummy src 触发依赖编译
COPY protocol-test-server/Cargo.toml protocol-test-server/Cargo.lock* ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN mkdir -p proto && echo 'syntax = "proto3";' > proto/test_service.proto
RUN cargo build --release 2>/dev/null || true

# 实际构建：覆盖 dummy src 和 proto，重新编译
COPY protocol-test-server/proto/ ./proto/
COPY protocol-test-server/src/ ./src/
COPY protocol-test-server/build.rs ./build.rs
RUN touch src/main.rs
RUN cargo build --release

# 编译 MCP stdio 二进制
RUN cargo build --release --bin mcp-stdio-server

# ============================================================
# Stage 2: Runtime — 精简运行时镜像
# ============================================================
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    openssl \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/protocol-test-server/target/release/protocol-test-server ./protocol-test-server
COPY --from=builder /usr/src/protocol-test-server/target/release/mcp-stdio-server ./mcp-stdio-server

# 构建时生成自签名 TLS 证书
RUN mkdir -p /app/certs && \
    openssl req -x509 -newkey rsa:2048 \
      -keyout /app/certs/server.key \
      -out /app/certs/server.crt \
      -days 3650 -nodes \
      -subj "/CN=localhost" \
      -addext "subjectAltName=DNS:localhost,IP:127.0.0.1,IP:0.0.0.0"

EXPOSE 9001 9002 9003 9004 9101 9102 8080 8081 50051 50052 50053 8085

CMD ["./protocol-test-server"]
