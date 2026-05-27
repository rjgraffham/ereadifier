FROM docker.io/library/rust:1.95-alpine as build

COPY . /work

WORKDIR /work
RUN cargo build --release

FROM scratch

LABEL org.opencontainers.image.source=https://github.com/rjgraffham/ereadifier
LABEL org.opencontainers.image.description="ereadifier container image"
LABEL org.opencontainers.image.licenses=MIT

EXPOSE 80

COPY --from=build /work/target/release/ereadifier /ereadifier

ENTRYPOINT [ "/ereadifier" ]
