#!/usr/bin/env bash

# if docker minio is already running, exit
if [ "$(docker ps -q -f name=minio)" ]; then
    echo "Minio is already running"
    exit 0
fi

docker run --name minio --rm -p 9000:9000 -p 9001:9001 --mount type=tmpfs,destination=/data  cgr.dev/chainguard/minio@sha256:bd014394a80898e68c149f2311fdf8d5a2c2f3bb2c33b9327ae6d02b4b065ae1 server /data --console-address ":9001" > /dev/null 2>&1 &
