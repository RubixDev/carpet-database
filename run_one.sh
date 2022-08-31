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

user_id="$(id -u)"

# build docker image
docker build -t "carpet-database-java-${java_version}" - << EOF
FROM eclipse-temurin:${java_version}
COPY --from=python:3.10 /usr/local /usr/local
COPY --from=python:3.10 /usr/bin /usr/bin
COPY --from=python:3.10 /usr/lib /usr/lib
COPY --from=python:3.10 /etc /etc

RUN [ "$user_id" = 0 ] || useradd -s /bin/bash -u "$user_id" -m user
RUN mkdir -p /app
RUN [ "$user_id" = 0 ] || chown -R user:user /app
WORKDIR /app
EOF

# run image
docker run --rm -u "$user_id" \
    -v "$PWD/data:/app/data" \
    -v "$PWD/cache/repos:/app/repos" \
    -v "$PWD/cache/gradle:/home/user/.gradle" \
    -v "$PWD/main.py:/app/run.py:ro" \
    -v "$PWD/printers:/app/printers:ro" \
    "carpet-database-java-${java_version}" \
    python run.py "${java_version}"
