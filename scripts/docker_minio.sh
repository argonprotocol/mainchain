#!/usr/bin/env bash
docker run --rm -p 9000:9000 -p 9001:9001 --mount type=tmpfs,destination=/data  minio/minio server /data --console-address ":9001"
