## Docker image

To build a single architecture docker image (change the version as required):

```shell
> docker build -t pactfoundation/pact-stub-server:0.6.0 --build-arg VERSION=0.6.0 .
```

To build a multi-architecture docker image:

Refer to https://www.docker.com/blog/multi-arch-build-and-images-the-simple-way/

Create a multi-arch builder

```shell
docker buildx create --name multiarch --use
```

Build images

```shell
export DOCKER_TAG=0.6.0
docker buildx build -t pactfoundation/pact-stub-server:$DOCKER_TAG --build-arg VERSION=$DOCKER_TAG --platform linux/amd64,linux/arm64 .
docker buildx build -t pactfoundation/pact-stub-server:latest --build-arg VERSION=$DOCKER_TAG --platform linux/amd64,linux/arm64 .
```

Build & Push images - will use cached images if available

```shell
export DOCKER_TAG=0.6.0
docker buildx build -t pactfoundation/pact-stub-server:$DOCKER_TAG --build-arg VERSION=$DOCKER_TAG --platform linux/amd64,linux/arm64 --push .
docker buildx build -t pactfoundation/pact-stub-server:latest --build-arg VERSION=$DOCKER_TAG --platform linux/amd64,linux/arm64 --push .
```