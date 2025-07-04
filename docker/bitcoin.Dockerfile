FROM debian:bullseye-slim

ARG VERSION=28.0
ENV VERSION=$VERSION

RUN apt-get update && apt-get install -y wget curl ca-certificates jq && \
    rm -rf /var/lib/apt/lists/*

RUN  <<END
    ARCH=$(dpkg --print-architecture)
    if [ "$ARCH" = "amd64" ]; then
        PLATFORM="x86_64-linux-gnu"
    elif [ "$ARCH" = "arm64" ]; then
        PLATFORM="aarch64-linux-gnu"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
    VERSION=${VERSION#v}
    wget https://bitcoincore.org/bin/bitcoin-core-$VERSION/bitcoin-$VERSION-$PLATFORM.tar.gz
    tar xzf bitcoin-$VERSION-$PLATFORM.tar.gz
    install -m 0755 -o root -g root -t /usr/local/bin bitcoin-$VERSION/bin/*
    rm -rf /opt/bitcoin-${VERSION}/bin/bitcoin-qt
END

COPY ./docker/init-bitcoin.sh init-bitcoin.sh
RUN chmod +x ./init-bitcoin.sh

CMD [ "/usr/local/bin/bitcoind"]
