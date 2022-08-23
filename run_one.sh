#!/bin/sh

# abort on errors
set -e

case "$1" in
    "8" | "16" | "18") java_version="$1" ;;
    *)
        echo "Specified java version must be 8, 16 or 18, defaulting to 18..."
        java_version="18"
        ;;
esac

# build docker image
docker build -t "carpet-database-java-${java_version}" - << EOF
FROM eclipse-temurin:${java_version}
COPY --from=python:3.10 /usr/local /usr/local
COPY --from=python:3.10 /usr/bin /usr/bin
COPY --from=python:3.10 /usr/lib /usr/lib
COPY --from=python:3.10 /etc /etc
EOF

# run image
docker run --rm -it \
    -v "$PWD/data:/data" \
    -v "$PWD/cache/repos:/repos" \
    -v "$PWD/cache/gradle:/root/.gradle" \
    -v "$PWD/main.py:/run.py:ro" \
    -v "$PWD/printers:/printers:ro" \
    "carpet-database-java-${java_version}" \
    python run.py "${java_version}"
