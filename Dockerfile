FROM rust:1.71.1 as planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.71.1 as cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY sqlx-data.json /app/sqlx-data.json
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:1.71.1 as builder
COPY . /app
WORKDIR /app
# set sqlx to offline mode to use the current sqlx-data instead of trying to connect to db
ENV SQLX_OFFLINE true
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo build --release

FROM gcr.io/distroless/cc-debian11 as runtime
WORKDIR /app
COPY --from=builder /app/target/release/splitwizz-service .
ENTRYPOINT [ "./splitwizz-service" ]
