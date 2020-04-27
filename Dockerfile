FROM rust:1.42@sha256:efb71dab7a9b8c90d1d4527da04b070f33517969300668676011d59c041e177d AS build

# create a new empty shell project
RUN USER=root cargo new --bin kubes-sidecar
WORKDIR /kubes-sidecar

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# build for release
RUN rm ./target/release/deps/kubes_sidecar*
RUN cargo build --release

RUN mkdir -p /build-out

RUN cp target/release/kubes-sidecar /build-out/

FROM ubuntu@sha256:bec5a2727be7fff3d308193cfde3491f8fba1a2ba392b7546b43a051853a341d

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get -y install ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

COPY --from=build /build-out/kubes-sidecar /

CMD /kubes-sidecar
