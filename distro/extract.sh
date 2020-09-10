#! /bin/bash

CID=$(docker create $1)
docker cp "$CID:$2" "$3"
docker rm $CID

