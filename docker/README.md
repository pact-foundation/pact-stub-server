## Docker image

To build a single architecture docker image (change the version as required):

```shell
> docker build -t pactfoundation/pact-stub-server:0.5.2 --build-arg VERSION=0.5.2 .
```

To build a multi-architecture docker image:

Refer to https://www.docker.com/blog/multi-arch-build-and-images-the-simple-way/

```shell
# AMD64
> docker build -t pactfoundation/pact-stub-server:0.5.2-amd64 --build-arg VERSION=0.5.2 --build-arg ARCH=amd64/ --build-arg BIN_ARCH=x86_64 .
> docker push pactfoundation/pact-stub-server:0.5.2-amd64

# ARM64V8
> docker build -t pactfoundation/pact-stub-server:0.5.2-arm64v8 --build-arg VERSION=0.5.2 --build-arg ARCH=arm64v8/ --build-arg BIN_ARCH=aarch64 .
> docker push pactfoundation/pact-stub-server:0.5.2-arm64v8

# Create Manifest
> docker manifest create pactfoundation/pact-stub-server:0.5.2 \
    --amend pactfoundation/pact-stub-server:0.5.2-amd64 \
    --amend pactfoundation/pact-stub-server:0.5.2-arm64v8
> docker manifest push pactfoundation/pact-stub-server:0.5.2
> docker manifest create pactfoundation/pact-stub-server:latest \
    --amend pactfoundation/pact-stub-server:0.5.2-amd64 \
    --amend pactfoundation/pact-stub-server:0.5.2-arm64v8
> docker manifest push pactfoundation/pact-stub-server:latest
```
