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
WORKDIR /app
ARG PROFILE="release"

COPY . .

RUN cargo build --profile $PROFILE --package rusty-runner-server --bin rusty-runner-server

# Copy binary to /bin/server for future use
RUN mkdir /app/bin
RUN cp target/$PROFILE/rusty-runner-server /app/bin/server
# Copy healthcheck binary
COPY --from=httpget /httpget /app/bin/httpget

FROM rust:slim AS runtime
LABEL name="rusty-remote-runner" \
      maintainer="info@meintest.software" \
      vendor="meinTest GmbH"

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

EXPOSE 8000
HEALTHCHECK --interval=30s --timeout=3s --retries=3 CMD ["/bin/httpget", "http://127.0.0.1:8000/health"]
ENTRYPOINT ["/bin/server", "--host=0.0.0.0"]