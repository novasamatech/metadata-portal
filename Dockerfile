FROM rust:1.66.1


# Install system packages
RUN apt-get update \
    && apt-get install -y \
        --no-install-recommends \
        libopencv-dev clang libclang-dev libssl-dev ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml .
COPY src/ .
COPY cli/ .
COPY public/ .

ENTRYPOINT ["cargo", "run", "--release"]
