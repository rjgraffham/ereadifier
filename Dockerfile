FROM docker.io/library/rust:1.95

LABEL org.opencontainers.image.source=https://github.com/rjgraffham/ereadifier
LABEL org.opencontainers.image.description="ereadifier container image"
LABEL org.opencontainers.image.licenses=MIT
LABEL org.opencontainers.image.version="0.3.2"

RUN --mount=type=bind,target=/source git clone /source /work

WORKDIR /work
RUN cargo build --release

EXPOSE 80

ENTRYPOINT [ "/work/target/release/ereadifier" ]