FROM rust:latest as builder
WORKDIR /app
ENV RUSTFLAGS='-C target-feature=+crt-static'
RUN git clone --recurse-submodules "https://github.com/ebesirik/irc2torrent.git" .
RUN cargo build --release --target x86_64-unknown-linux-gnu

FROM jesec/rtorrent-flood:latest as runner
LABEL authors="ebesirik"

USER root
RUN mkdir -p /config/.config/irc2torrent
RUN chown -R download:download /config/.config/irc2torrent
USER download
COPY --chown=download:download --from=builder /app/target/x86_64-unknown-linux-gnu/release/irc2torrent /app/

WORKDIR /app
ENTRYPOINT ["/bin/sh", "-c", "/app/irc2torrent& /sbin/tini -- flood"]
