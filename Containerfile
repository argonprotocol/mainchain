FROM docker.io/library/ubuntu:22.04

# show backtraces
ENV RUST_BACKTRACE 1
ARG BIN=argon-node
ARG TARGETARCH
ENV BIN=${BIN}
# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
		ca-certificates && \
# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
# add user and link ~/.local/share/argon to /data
	useradd -m -u 1000 -U -s /bin/sh -d /argon argon && \
	mkdir -p /data /argon/.local/share && \
	chown -R argon:argon /data && \
    ln -s /data /argon/.local/share/${BIN}

USER argon
# copy the compiled binary to the container
COPY --chown=argon:argon --chmod=774 ${TARGETARCH}/${BIN} /usr/bin/${BIN}

# check if executable works in this container
RUN /usr/bin/${BIN} --version

ENTRYPOINT ["/bin/sh", "-c", "/usr/bin/$BIN \"$@\"", "--"]
