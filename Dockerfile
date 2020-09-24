FROM rust:1.46-alpine AS Builder

RUN apk add --no-cache musl-dev

WORKDIR /src/

# copy source code
COPY Cargo.toml ./
COPY examples ./examples
COPY vrp-core ./vrp-core
COPY vrp-scientific ./vrp-scientific
COPY vrp-pragmatic ./vrp-pragmatic
COPY vrp-cli ./vrp-cli

RUN cargo test
RUN cargo build --release


FROM alpine:3.12

ENV SOLVER_USER=user
ENV SOLVER_DIR=/solver
RUN addgroup $SOLVER_USER && adduser -S $SOLVER_USER -G $SOLVER_USER

RUN mkdir $SOLVER_DIR
COPY --from=Builder /src/target/release/vrp-cli $SOLVER_DIR/vrp-cli
RUN chown -R $SOLVER_USER:$SOLVER_USER $SOLVER_DIR/vrp-cli

USER $SOLVER_USER
WORKDIR $SOLVER_DIR
