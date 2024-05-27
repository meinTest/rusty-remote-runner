# syntax=docker/dockerfile:1

# This dockerfile builds the rusty-runner-server service as a container.
# The service starts automatically when running the container on
# - host: 0.0.0.0
# - port: 1337
#
# Example:
#
#   $ docker run rusty-runner-server
#

FROM rust:latest as builder
LABEL authors="oeju1"
WORKDIR /app
ARG APP_NAME=rusty-runner-server
ARG APP_BIN=$APP_NAME
#ARG CLI_TARGET=""
ARG CLI_TARGET="--release"
#ARG TARGET="debug"
ARG TARGET="release"

COPY . .

# Build the application.
# Leverage a cache mount to /usr/local/cargo/registry/
# for downloaded dependencies, a cache mount to /usr/local/cargo/git/db
# for git repository dependencies, and a cache mount to /app/target/ for
# compiled dependencies which will speed up subsequent builds.
# Leverage a bind mount to the src directory to avoid having to copy the
# source code into the container. Once built, copy the executable to an
# output directory before the cache mounted /app/target is unmounted.
#RUN --mount=type=bind,source=$APP_NAME/src,target=$APP_NAME/src \
#    --mount=type=bind,source=$APP_NAME/Cargo.toml,target=$APP_NAME/Cargo.toml \
#    --mount=type=bind,source=$APP_NAME/Cargo.lock,target=$APP_NAME/Cargo.lock \
#    --mount=type=cache,target=$APP_NAME/target/ \
#    --mount=type=cache,target=/usr/local/cargo/git/db \
#    --mount=type=cache,target=/usr/local/cargo/registry/ \
RUN cargo build $CLI_TARGET --package $APP_NAME --bin $APP_BIN

# Copy binary to /bin/server for future use
RUN cp target/$TARGET/$APP_BIN /bin/server

FROM rust:slim as runtime
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

COPY --from=builder /bin/server /bin/

HEALTHCHECK --interval=1m --timeout=3s CMD curl -f http://localhost:1337/health || exit 1
ENTRYPOINT ["/bin/server"]