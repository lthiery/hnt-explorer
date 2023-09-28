####################################################################################################
## Builder
####################################################################################################
FROM debian:buster-slim AS builder

RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y curl build-essential

# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Create appuser
ENV USER=hnt-explorer
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


WORKDIR /hnt-explorer

COPY ./ .

RUN cargo build --release

####################################################################################################
## Final image
####################################################################################################
FROM debian:buster-slim

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /hnt-explorer

# Copy our build
COPY --from=builder /hnt-explorer/target/release/hnt-explorer ./

# Use an unprivileged user.
USER hnt-explorer:hnt-explorer

CMD ["/hnt-explorer/hnt-explorer", "server"]
