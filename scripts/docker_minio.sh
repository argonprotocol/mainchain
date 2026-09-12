#!/usr/bin/env bash

# if docker minio is already running, exit
if [ "$(docker ps -q -f name=minio)" ]; then
    echo "Minio is already running"
    exit 0
fi

docker run --name minio --rm -p 9000:9000 -p 9001:9001 --mount type=tmpfs,destination=/data  quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z@sha256:14cea493d9a34af32f524e538b8346cf79f3321eff8e708c1e2960462bd8936e server /data --console-address ":9001" > /dev/null 2>&1 &
