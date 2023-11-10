FROM rust:latest as builder
WORKDIR /app
ENV RUSTFLAGS='-C target-feature=+crt-static'
RUN git clone "https://github.com/ebesirik/irc2torrent.git" .
RUN #cargo install cargo-bundle
RUN cargo build --release --target x86_64-unknown-linux-gnu
#RUN cargo bundle --release
#CMD ["ls", "-lah", "/app/target/release/bundle/deb/"]

FROM jesec/rtorrent-flood:latest as runner
LABEL authors="ebesirik"
#CMD ["/bin/bash", "mkdir", "/app"]
USER root
RUN mkdir -p /config/.config/irc2torrent
RUN chown -R download:download /config/.config/irc2torrent
USER download
COPY --chown=download:download --from=builder /app/target/x86_64-unknown-linux-gnu/release/irc2torrent /app/
COPY --chown=download:download ./irc.defaults.toml /app/irc.defaults.toml
COPY --chown=download:download ./options.toml /app/options.toml
#RUN chmod +x /app/irc2torrent
#RUN chown -c download:download /app/irc2torrent
WORKDIR /app
ENTRYPOINT ["/bin/sh", "-c", "/app/irc2torrent& /sbin/tini -- flood"]
#ENTRYPOINT ["/bin/bash"]
#ENTRYPOINT ["ls", "-lah", "/app/"]