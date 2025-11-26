#!/bin/bash
# Build script for Linux targets using vcpkg with fallback support

cd ..

docker build -t seeu-desktop-linux-builder -f Dockerfile.uos .
docker create --name seeu-temp-container seeu-desktop-linux-builder
docker cp seeu-temp-container:/output/seeu_desktop dist/linux/
docker rm seeu-temp-container