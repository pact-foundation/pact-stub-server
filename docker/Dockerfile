FROM debian:11-slim as builder
ARG TARGETARCH
ARG VERSION=0.5.3

RUN apt-get update && apt-get install -y wget
RUN if [ "$TARGETARCH" = "amd64" ]; then \
        BIN_ARCH="x86_64"; \
    elif [ "$TARGETARCH" = "arm64" ]; then \
        BIN_ARCH="aarch64"; \
    else \
        echo "Invalid TARGET_ARCH"; \
        exit 1; \
    fi \
    && wget -O pact-stub-server.gz https://github.com/pact-foundation/pact-stub-server/releases/download/v${VERSION}/pact-stub-server-linux-${BIN_ARCH}.gz
RUN gunzip pact-stub-server.gz
RUN chmod +x pact-stub-server

FROM --platform=linux/${TARGETARCH} debian:11-slim
WORKDIR /app
COPY --from=builder /pact-stub-server /app/
EXPOSE 8080
ENTRYPOINT ["./pact-stub-server"]
CMD ["--help"]
