FROM rust:1.66.1


# Install system packages
RUN apt-get update \
    && apt-get install -y \
        --no-install-recommends \
        libopencv-dev clang libclang-dev libssl-dev ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN apt remove cmdtest
RUN apt remove yarn
RUN curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | apt-key add -
RUN echo "deb https://dl.yarnpkg.com/debian/ stable main" | tee /etc/apt/sources.list.d/yarn.list
RUN apt-get update
RUN apt-get install yarn -y

COPY . /

RUN make collector; exit 0
RUN yarn 

ENTRYPOINT ["yarn", "start"]
