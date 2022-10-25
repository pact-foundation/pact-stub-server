ARG ARCH=
FROM debian:11-slim as builder
ARG VERSION=0.5.0
ARG BIN_ARCH=x86_64

RUN apt-get update && apt-get install -y wget
RUN wget -O pact-stub-server.gz https://github.com/uglyog/pact-stub-server/releases/download/v${VERSION}/pact-stub-server-linux-${BIN_ARCH}.gz
RUN gunzip pact-stub-server.gz
RUN chmod +x pact-stub-server

FROM ${ARCH}debian:11-slim
WORKDIR /app
COPY --from=builder /pact-stub-server /app/
EXPOSE 8080
ENTRYPOINT ["./pact-stub-server"]
CMD ["--help"]
