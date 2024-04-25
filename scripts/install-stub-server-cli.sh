#!/usr/bin/env sh

set -e
set -x

VERSION="0.5.3"

mkdir -p ~/.pact/bin
case "$(uname -s)" in

   Darwin)
     echo '== Installing stub server CLI for Mac OSX =='
     if [ "$(uname -m)" = "arm64" ]; then
        curl -L -o ~/.pact/bin/pact-stub-server.gz https://github.com/pact-foundation/pact-stub-server/releases/download/v${VERSION}/pact-stub-server-osx-aarch64.gz
     else
        curl -L -o ~/.pact/bin/pact-stub-server.gz https://github.com/pact-foundation/pact-stub-server/releases/download/v${VERSION}/pact-stub-server-osx-x86_64.gz
     fi
     gunzip -N -f ~/.pact/bin/pact-stub-server.gz
     chmod +x ~/.pact/bin/pact-stub-server
     ;;

   Linux)
     echo '== Installing stub server CLI for Linux =='
     if [ "$(uname -m)" = "aarch64" ]; then
      curl -L -o ~/.pact/bin/pact-stub-server.gz https://github.com/pact-foundation/pact-stub-server/releases/download/v${VERSION}/pact-stub-server-linux-aarch64.gz
     else
      curl -L -o ~/.pact/bin/pact-stub-server.gz https://github.com/pact-foundation/pact-stub-server/releases/download/v${VERSION}/pact-stub-server-linux-x86_64.gz
     fi
     gunzip -N -f ~/.pact/bin/pact-stub-server.gz
     chmod +x ~/.pact/bin/pact-stub-server
     ;;

   CYGWIN*|MINGW32*|MSYS*|MINGW*)
     echo '== Installing stub server CLI for MS Windows =='
     if [ "$(uname -m)" = "aarch64" ]; then
      curl -L -o ~/.pact/bin/pact-stub-server.exe.gz https://github.com/pact-foundation/pact-stub-server/releases/download/v${VERSION}/pact-stub-server-windows-aarch64.exe.gz
     else
      curl -L -o ~/.pact/bin/pact-stub-server.exe.gz https://github.com/pact-foundation/pact-stub-server/releases/download/v${VERSION}/pact-stub-server-windows-x86_64.exe.gz
     fi
     gunzip -N -f ~/.pact/bin/pact-stub-server.exe.gz
     chmod +x ~/.pact/bin/pact-stub-server.exe
     ;;

   *)
     echo "ERROR: $(uname -s) is not a supported operating system"
     exit 1
     ;;
esac