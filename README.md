# Standalone Pact Stub Server

[![Build Status](https://travis-ci.org/uglyog/pact-stub-server.svg?branch=master)](https://travis-ci.org/uglyog/pact-stub-server)
[![Windows Build status](https://ci.appveyor.com/api/projects/status/vigbo2qdyk9x7mo9?svg=true)](https://ci.appveyor.com/project/uglyog/pact-stub-server)

This project provides a server that can generate responses based on pact files. It is a single executable binary. 
It implements the [V3 Pact specification](https://github.com/pact-foundation/pact-specification/tree/version-3).

[Online rust docs](https://docs.rs/crate/pact-stub-server/0.2.2)

The stub server works by taking all the interactions (requests and responses) from a number of pact files. 
For each interaction, it will compare any incoming request against those defined in the pact files. If there is a match 
(based on method, path and query parameters), it will return the response from the pact file.

## Command line interface

The pact stub server is bundled as a single binary executable `pact-stub-server`. Running this with out any options displays the standard help.

```console
pact-stub-server v0.2.2
Pact Stub Server

USAGE:
    pact-stub-server [FLAGS] [OPTIONS] --dir <dir>... --file <file>... --url <url>...

FLAGS:
    -o, --cors            Automatically respond to OPTIONS requests and return default CORS headers
    -h, --help            Prints help information
        --insecure-tls    Disables TLS certificate validation
    -v, --version         Prints version information

OPTIONS:
    -d, --dir <dir>...                       Directory of pact files to verify (can be repeated)
    -f, --file <file>...                     Pact file to verify (can be repeated)
    -l, --loglevel <loglevel>                Log level (defaults to info) [possible values: error, warn,
                                             info, debug, trace, none]
    -p, --port <port>                        Port to run on (defaults to random port assigned by the OS)
    -s, --provider-state <provider-state>    Provider state regular expression to filter the responses by
    -u, --url <url>...                       URL of pact file to verify (can be repeated)
        --user <user>                        User and password to use when fetching pacts from URLS in user:password
                                             form

```

## Options

### Log Level

You can control the log level with the `-l, --loglevel <loglevel>` option. It defaults to info, and the options that you can specify are: error, warn, info, debug, trace, none.

### CORS pre-flight requests

If you specify the `-o, --cors` option, then any un-matched OPTION request will result in a default 200 response.

### Pact File Sources

You can specify the pacts to verify with the following options. They can be repeated to set multiple sources.

| Option | Type | Description |
|--------|------|-------------|
| `-f, --file <file>` | File | Loads a pact from the given file |
| `-u, --url <url>` | URL | Loads a pact from a URL resource |
| `-d, --dir <dir>` | Directory | Loads all the pacts from the given directory |

*Note:* For URLs that are authenticated, you can use the `--user` option to set the username and password.

#### Disabling TLS certificate validation

If you need to load pact files from a HTTPS URL that is using a self-signed certificate, you can use the `--insecure-tls`
flag to disable the TLS certificate validation. WARNING: this disables all certificate validations, including expired
certificates.

### Filtering interactions by provider state

You can filter the interactions by provider state by supplying the `--provider-state` option. This takes a regular
expression that is applied to all interactions before the requests are matched.

### Server Options

The running server can be controlled with the following options:

| Option | Description |
|--------|-------------|
| `-p, --port <port>` | The port to bind to. If not specified, a random port will be allocated by the operating system. |
