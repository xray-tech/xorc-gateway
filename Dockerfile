FROM alpine:edge
MAINTAINER Julius de Bruijn <julius.debruijn@360dialog.com>

WORKDIR /usr/src
ENV USER root
ENV RUST_BACKTRACE 1

RUN mkdir -p /usr/src/xorc-gateway
RUN mkdir -p /etc/xorc-gateway
COPY Cargo.toml Cargo.lock build.rs /usr/src/xorc-gateway/
COPY src /usr/src/xorc-gateway/src
COPY third_party /usr/src/xorc-gateway/third_party
COPY third_party/events /usr/src/xorc-gateway/third_party/events

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /usr/src/xorc-gateway

RUN apk --no-cache add libgcc rust cargo file openssl openssl-dev protobuf \
    musl-dev libffi-dev ca-certificates make automake gnutls gnutls-dev bash g++ make python

ENV PROTOC /usr/bin/protoc
ENV PROTOC_INCLUDE /usr/include

RUN cargo build --release
RUN mv target/release/xorc-gateway /bin
RUN chmod a+x /bin/xorc-gateway
RUN rm -rf /usr/src/xorc-gateway

COPY config/config.toml.tests /etc/xorc-gateway/config.toml
ENV CONFIG "/etc/xorc-gateway/config.toml"

WORKDIR /


CMD "/bin/xorc-gateway"