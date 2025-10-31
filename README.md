# Standalone Pact Stub Server

[![Build](https://github.com/uglyog/pact-stub-server/workflows/Build/badge.svg)](https://github.com/uglyog/pact-stub-server/actions?query=workflow%3ABuild)

This project provides a server that can generate responses based on pact files. It is a single executable binary. 
It implements the [V4 Pact specification](https://github.com/pact-foundation/pact-specification/tree/version-4).

[Online rust docs](https://docs.rs/crate/pact-stub-server)

[![pulls](https://badgen.net/docker/pulls/pactfoundation/pact-stub-server?icon=docker&label=pulls)](https://hub.docker.com/r/pactfoundation/pact-stub-server)
[![stars](https://badgen.net/docker/stars/pactfoundation/pact-stub-server?icon=docker&label=stars)](https://hub.docker.com/r/pactfoundation/pact-stub-server)

[![size: amd64](https://badgen.net/docker/size/pactfoundation/pact-stub-server/latest/amd64?icon=docker&label=size%3Aamd64)](https://hub.docker.com/r/pactfoundation/pact-stub-server)
[![size: arm64](https://badgen.net/docker/size/pactfoundation/pact-stub-server/latest/arm64?icon=docker&label=size%3Aarm64)](https://hub.docker.com/r/pactfoundation/pact-stub-server)

The stub server works by taking all the interactions (requests and responses) from a number of pact files. 
For each interaction, it will compare any incoming request against those defined in the pact files. If there is a match 
(based on method, path and query parameters), it will return the response from the pact file.

**Note:** The stub server was designed to supporting prototyping of mobile applications by stubbing out the
backend servers. It will always try to return a response, even when there is not an extract match with the
pact files.

## Command line interface

The pact stub server is bundled as a single binary executable `pact-stub-server`. Running this without any options displays the standard help.

```console,ignore
$ pact-stub-server
Pact Stub Server 0.5.3

Usage: pact-stub-server [OPTIONS]

Options:
  -l, --loglevel <loglevel>
          Log level (defaults to info) [default: info] [possible values: error, warn, info, debug, trace, none]
  -f, --file <file>
          Pact file to load (can be repeated)
  -d, --dir <dir>
          Directory of pact files to load (can be repeated)
  -e, --extension <ext>
          File extension to use when loading from a directory (default is json)
  -u, --url <url>
          URL of pact file to fetch (can be repeated)
  -b, --broker-url <broker-url>
          URL of the pact broker to fetch pacts from [env: PACT_BROKER_BASE_URL=]
      --user <user>
          User and password to use when fetching pacts from URLS or Pact Broker in user:password form
  -t, --token <token>
          Bearer token to use when fetching pacts from URLS or Pact Broker
  -p, --port <port>
          Port to run on (defaults to random port assigned by the OS)
  -o, --cors
          Automatically respond to OPTIONS requests and return default CORS headers
      --cors-referer
          Set the CORS Access-Control-Allow-Origin header to the Referer
      --insecure-tls
          Disables TLS certificate validation
  -s, --provider-state <provider-state>
          Provider state regular expression to filter the responses by
      --provider-state-header-name <provider-state-header-name>
          Name of the header parameter containing the provider state to be used in case multiple matching interactions are found
      --empty-provider-state
          Include empty provider states when filtering with --provider-state
      --consumer-name <consumer-name>
          Consumer name or regex to use to filter the Pacts fetched from the Pact broker (can be repeated)
      --provider-name <provider-name>
          Provider name or regex to use to filter the Pacts fetched from the Pact broker (can be repeated)
  -v, --version
          Print version information
  -h, --help
          Print help

```

## Options

### Log Level

You can control the log level with the `-l, --loglevel <loglevel>` option. It defaults to info, and the options that you can specify are: error, warn, info, debug, trace, none.

### CORS pre-flight requests

If you specify the `-o, --cors` option, then any un-matched OPTION request will result in a default 200 response. By default the 
Access-Control-Allow-Origin header will be set to `*`. If you provide the `--cors-referer` flag, then it will be set to the
value of the Referer header from the request.

### Pact File Sources

You can specify the pacts to verify with the following options. They can be repeated to set multiple sources.

| Option | Type | Description |
|--------|------|-------------|
| `-f, --file <file>` | File | Loads a pact from the given file |
| `-u, --url <url>` | URL | Loads a pact from a URL resource |
| `-d, --dir <dir>` | Directory | Loads all the pacts from the given directory |
| `-b, --broker-url <url>` | URL | Loads all the latest pacts from the Pact Broker |

*Note:* For URLs and Pact Brokers that are authenticated, you can use the `--user` option to set the username and password or the
`--token` to use a bearer token.

#### Disabling TLS certificate validation

If you need to load pact files from a HTTPS URL that is using a self-signed certificate, you can use the `--insecure-tls`
flag to disable the TLS certificate validation. WARNING: this disables all certificate validations, including expired
certificates.

### Filtering interactions by provider state

You can filter the interactions by provider state by supplying the `--provider-state` option. This takes a regular
expression that is applied to all interactions before the requests are matched.

### Filtering interactions by consumer and provider name (Pact Broker)

For Pacts fetched from a Pact broker, you can filter the Pacts by the consumer and/or provider names using: 

```ignore
 --consumer-names <consumer-names>...
            Consumer names to use to filter the Pacts fetched from the Pact broker
            
 --provider-names <provider-names>...
            Provider names to use to filter the Pacts fetched from the Pact broker
```

### Server Options

The running server can be controlled with the following options:

| Option | Description |
|--------|-------------|
| `-p, --port <port>` | The port to bind to. If not specified, a random port will be allocated by the operating system. |

### Watch mode

The Pact Stub Server now supports a watch mode that automatically monitors pact files and directories for changes and reloads them without restarting the server. This feature is particularly useful during development when pact files are frequently updated.

#### Usage

To enable watch mode, use the `--watch` or `-w` flag:

```bash
# Watch a single pact file
pact-stub-server --file path/to/pact.json --watch --port 8080

# Watch a directory of pact files
pact-stub-server --dir path/to/pacts --watch --port 8080

# Watch multiple files and directories
pact-stub-server --file pact1.json --file pact2.json --dir pacts_dir --watch --port 8080
```

#### Supported Source Types

Watch mode supports the following pact source types:

- **File sources** (`--file`): Individual pact files are monitored for changes
- **Directory sources** (`--dir`): Entire directories are monitored recursively for changes to pact files

**Note**: URL sources (`--url`) and Pact Broker sources (`--broker-url`) are not supported for watching as they are remote resources.

#### File System Events

The watcher responds to the following types of file system events:

- File modifications (content changes)
- File creation (new pact files added)
- File deletion (pact files removed)
- Directory changes (new files added to watched directories)

#### Limitations

- Only works with local file and directory sources
- URL and Pact Broker sources cannot be watched
- Requires file system notifications to be available on the host system
- Memory usage may be slightly higher due to shared state management

## Docker

### Usage 

Docker images are published to official registries, see [here](#docker---supported-registries)

Example of using it:

```console,ignore
# Create a Stub API
docker pull pactfoundation/pact-stub-server
docker run -t -p 8080:8080 -v "$(pwd)/pacts/:/app/pacts" pactfoundation/pact-stub-server -p 8080 -d pacts

# Test your stub endpoints
curl -v $(docker-machine ip $(docker-machine active)):8080/bazbat
curl -v $(docker-machine ip $(docker-machine active)):8080/foobar
```

### Docker - Supported Platforms

Multi-platform images are available, and can be used cross-platform by setting the `platform` flag.

- `--platform=linux/amd64` 
- `--platform=linux/arm64` 

```console,ignore
  docker run --platform=linux/arm64 -t -p 8080:8080 -v "$(pwd)/pacts/:/app/pacts" pactfoundation/pact-stub-server -p 8080 -d pacts
```

### Docker - Supported Registries

Docker images are published to multiple registries 

- [DockerHub Image](https://hub.docker.com/r/pactfoundation/pact-stub-server)
- [GitHub Container Image](https://github.com/pact-foundation/pact-stub-server/pkgs/container/pact-stub-server)

#### Docker - DockerHub

```console,ignore
  docker pull pactfoundation/pact-stub-server
```

#### Docker - GitHub Container Registry

```console,ignore
  docker pull ghcr.io/pact-foundation/pact-stub-server
```

## Compatibility

<details><summary>Supported Platforms</summary>

| OS      | Architecture | Supported  | Pact Stub Server Version |
| ------- | ------------ | ---------  | ---------------- |
| MacOS   | x86_64       | ✅         | All              |
| Linux   | x86_64       | ✅         | All              |
| Windows | x86_64       | ✅         | All              |
| MacOS   | arm64        | ✅         | >=0.5.2          |
| Linux   | arm64        | ✅         | >=0.5.2          |
| Windows | arm64        | ✅         | >=0.6.0          |
| Alpine  | x86_64       | ✅         | >=0.6.0          |
| Alpine  | arm64        | ✅         | >=0.6.0          |

_Note:_ From v0.6.0, Linux executables are statically built with `musl` and as designed to work against `glibc` (eg, Debian) and `musl` (eg, Alpine) based distos.

</details>
