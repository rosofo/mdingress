FROM rust:1-bullseye as build

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src src
RUN ls
RUN --mount=type=cache,target=/root/.cargo cargo build --release --locked

FROM ubuntu

RUN apt-get update && apt-get install -y --no-install-recommends \
    avahi-utils \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /app/target/release/mdingress /bin/mdingress