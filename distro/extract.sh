#! /bin/bash

CID=$(docker create $1)
docker cp -L "$CID:$2" "$3"
docker rm $CID

