# Stage 1: Static musl build
FROM alpine:3.21 AS builder
RUN apk add --no-cache curl bash gcc musl-dev cmake make g++ openssl-dev openssl-libs-static perl curl-dev zlib-dev zlib-static git
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:$PATH"
WORKDIR /workspace
COPY . .
RUN cd src-tauri && cargo build -p mpe-cli --release

# Stage 2: Minimal runtime image
FROM alpine:3.21
RUN apk add --no-cache ca-certificates tzdata \
    && addgroup -g 10001 -S mpe \
    && adduser -u 10001 -S mpe -G mpe -h /home/mpe
COPY --from=builder --chown=mpe:mpe /workspace/src-tauri/target/release/mpe-cli /usr/local/bin/mpe
USER mpe:mpe
WORKDIR /home/mpe
EXPOSE 9527
HEALTHCHECK --interval=10s --timeout=3s --retries=3 \
  CMD nc -z 127.0.0.1 9527 || exit 1
ENTRYPOINT ["/usr/local/bin/mpe", "serve"]
CMD ["--flow", "/flows", "--port", "9527"]
