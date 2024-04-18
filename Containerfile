FROM docker.io/library/ubuntu:22.04

# show backtraces
ENV RUST_BACKTRACE 1
ARG BIN=ulx-node
# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
		ca-certificates && \
# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
# add user and link ~/.local/share/ulixee to /data
	useradd -m -u 1000 -U -s /bin/sh -d /ulixee ulixee && \
	mkdir -p /data /ulixee/.local/share && \
	chown -R ulixee:ulixee /data && \
    ln -s /data /ulixee/.local/share/${BIN}

USER ulixee
# copy the compiled binary to the container
COPY --chown=ulixee:ulixee --chmod=774 ${BIN} /usr/bin/${BIN}

# check if executable works in this container
RUN /usr/bin/${BIN} --version

CMD /usr/bin/${BIN}