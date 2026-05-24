FROM docker.io/library/rust:1.95

RUN --mount=type=bind,target=/source git clone /source /work

WORKDIR /work
RUN cargo build --release

EXPOSE 80

ENTRYPOINT [ "/work/target/release/ereadifier" ]