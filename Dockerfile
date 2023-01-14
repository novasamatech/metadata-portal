FROM rust:1.60

# Build time options to avoid dpkg warnings and help with reproducible builds.
ENV DEBIAN_FRONTEND=noninteractive \

# Install system packages
RUN apt-get update \
    && apt-get install -y \
        --no-install-recommends \
        libopencv-dev clang libclang-dev libssl-dev ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*


ENTRYPOINT ["cargo", "run", "--release"]
