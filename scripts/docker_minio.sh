#!/usr/bin/env bash

# if docker minio is already running, exit
if [ "$(docker ps -q -f name=minio)" ]; then
    echo "Minio is already running"
    exit 0
fi

docker run --name minio --rm -p 9000:9000 -p 9001:9001 --mount type=tmpfs,destination=/data  minio/minio server /data --console-address ":9001" > /dev/null 2>&1 &
