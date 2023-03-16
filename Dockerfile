FROM docker.io/alpine:latest as builder
RUN apk add --no-cache \
      cargo \
      build-base \
      openssl-dev \
      git
COPY . /app
WORKDIR /app
RUN cargo build --release

FROM docker.io/alpine:latest
RUN apk add --no-cache \
      libgcc \
      libssl3 \
  && mkdir -p /opt/hcloud-prom-filesd
WORKDIR /opt/hcloud-prom-filesd
COPY --from=builder /app/target/release/hcloud-prom-filesd /usr/local/bin/hcloud-prom-filesd
CMD ["/usr/local/bin/hcloud-prom-filesd"]
