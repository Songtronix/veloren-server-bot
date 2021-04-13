# Credit goes to https://www.lpalmieri.com/posts/2020-11-01-zero-to-production-5-how-to-deploy-a-rust-application/#3-8-optimising-our-docker-image

# Check for modified dependencies
FROM lukemathwalker/cargo-chef as planner
WORKDIR app
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json

# Cache dependencies
FROM lukemathwalker/cargo-chef as cacher
WORKDIR app
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application! 
RUN cargo chef cook --release --recipe-path recipe.json

# Build Bot
FROM rust AS builder
WORKDIR app
# Copy over the cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
COPY . .
# Build our application, leveraging the cached deps!
RUN cargo build --release

# Veloren Server Bot Runtime Environment.
# Requires git, git-lfs, rustup and whatever Veloren gameserver depends on.
FROM ubuntu:18.04 as runtime

RUN apt-get update
RUN export DEBIAN_FRONTEND=noninteractive
RUN apt-get install -y --no-install-recommends --assume-yes \
        ca-certificates \
        build-essential \
        curl \
        git \
        git-lfs

RUN git lfs install

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile=minimal; \
        . /root/.cargo/env; \
        rustup default stable;

COPY --from=builder /app/target/release/veloren_server_bot .

ENV RUST_BACKTRACE=1
ENV PATH="/root/.cargo/bin:${PATH}"
CMD [ "./veloren_server_bot" ]
