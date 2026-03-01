FROM rust:1-bullseye as build

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src src
RUN ls
RUN cargo build --release

FROM ubuntu

WORKDIR /app
COPY --from=build /app/target/release/mdingress /bin/mdingress