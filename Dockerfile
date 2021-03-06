FROM rust:latest
MAINTAINER Julius de Bruijn <julius@nauk.io>

WORKDIR /usr/src
ENV USER root
ENV RUST_BACKTRACE 1

RUN apt-get -y update
RUN apt-get -y install libssl-dev protobuf-compiler libffi-dev build-essential python

ENV PROTOC /usr/bin/protoc
ENV PROTOC_INCLUDE /usr/include

RUN mkdir -p /usr/src/xorc-gateway
RUN mkdir -p /etc/xorc-gateway
COPY Cargo.toml Cargo.lock build.rs /usr/src/xorc-gateway/
COPY src /usr/src/xorc-gateway/src
COPY third_party /usr/src/xorc-gateway/third_party
COPY third_party/events /usr/src/xorc-gateway/third_party/events
COPY config /usr/src/xorc-gateway/config

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /usr/src/xorc-gateway
RUN cargo build --release

RUN cp target/release/xorc-gateway /bin
RUN chmod a+x /bin/xorc-gateway

CMD "/bin/xorc-gateway"
