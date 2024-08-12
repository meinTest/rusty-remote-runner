# syntax=docker/dockerfile:1

# This dockerfile builds the rusty-runner-server service as a container.
# The service starts automatically when running the container on
# - host: 127.0.0.1
# - port: 8000
#
# Example:
#
#   $ docker run rusty-runner-server
#

# For the healthcheck
FROM ghcr.io/cryptaliagy/httpget:latest AS httpget

FROM rust:latest AS builder
LABEL authors="oeju1"
WORKDIR /app
ARG APP_NAME=rusty-runner-server
ARG APP_BIN=$APP_NAME
#ARG CLI_TARGET=""
ARG CLI_TARGET="--release"
#ARG TARGET="debug"
ARG TARGET="release"

COPY . .

RUN cargo build $CLI_TARGET --package $APP_NAME --bin $APP_BIN

# Copy binary to /bin/server for future use
RUN mkdir /app/bin
RUN cp target/$TARGET/$APP_BIN /app/bin/server
# Copy healthcheck binary
COPY --from=httpget /httpget /app/bin/httpget


FROM rust:slim AS runtime
LABEL authors="oeju1"

# Create a non-privileged user that the app will run under.
# See https://docs.docker.com/go/dockerfile-user-best-practices/
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

COPY --from=builder /app/bin /bin

HEALTHCHECK --interval=30s --timeout=3s --retries=3 CMD ["/bin/httpget", "http://127.0.0.1:8000/health"]
ENTRYPOINT ["/bin/server", "--host=0.0.0.0"]