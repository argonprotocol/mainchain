# Argon RPC

This image wraps the upstream Acala `subway` binary with Argon's checked-in RPC config and method
allowlist.

## How it works

1. [`Containerfile`](./Containerfile) builds `subway` from the pinned upstream Acala release in
   `UPSTREAM_SUBWAY_REF`.
2. The runtime image copies [`config.yml`](./config.yml) into `/config/argon-rpc.yml`.
3. The runtime image copies [`rpcs.yml`](./rpcs.yml) into `/config/rpcs.yml`.
4. The image starts with:

   ```sh
   subway --config /config/argon-rpc.yml
   ```

5. Subway expands `${...}` placeholders inside the YAML before parsing it, so we can keep one
   checked-in config template and tune it with container environment variables.
6. By default the gateway proxies to `ws://archive-node:9944`. Backup upstreams can be added with
   `ARGON_RPC_ENDPOINTS`.

## Files

- [`Containerfile`](./Containerfile): builds the pinned upstream `subway` binary and packages the
  checked-in RPC config and allowlist.
- [`config.yml`](./config.yml): the main Subway config template with env-driven defaults.
- [`rpcs.yml`](./rpcs.yml): the allowed RPC surface, cache settings, and subscription middleware
  behavior.

## Compose behavior

In [`dev.docker-compose.yml`](../../dev.docker-compose.yml), the `archive-rpc` service:

- builds locally as `ghcr.io/argonprotocol/argon-rpc:${VERSION:-dev}`
- listens on container port `9944`
- publishes that port to `${RPC_PORT:-9944}` on the host
- uses the baked-in `/config/argon-rpc.yml` and `/config/rpcs.yml`
- passes through optional host overrides with `ARGON_RPC_*`

The raw archive node RPC host port is controlled separately with `ARCHIVE_NODE_RPC_PORT`. Its
default is `0`, so Docker assigns an ephemeral host port and the main developer entrypoint stays on
the gateway.

## Versioning

For published images, the immutable tag should be Argon-owned while still showing the upstream
Subway base:

- `v0.1.2-argon.1`
- `v0.1.2-argon.2`
- `v0.1.3-argon.1`

That means:

- bump `argon.N` when we change Argon packaging, config, or RPC policy on top of the same upstream
  Subway release
- reset to `.1` when we move to a new upstream Acala Subway tag

The current dev compose stack still builds and tags locally with `${VERSION:-dev}` because it is not
consuming a published `argon-rpc` image yet.

## Environment variables

The compose file forwards these variable names so you can override them from your shell or `.env`
file. If a value is unset or blank, the `${...:-default}` expression in the baked-in
[`config.yml`](./config.yml) falls back to the checked-in default.

### Compose-level variables

| Variable                | Default | Purpose                                                 |
| ----------------------- | ------- | ------------------------------------------------------- |
| `VERSION`               | `dev`   | Image tag used for the local build tag in compose.      |
| `RPC_PORT`              | `9944`  | Host port for the gateway itself.                       |
| `ARCHIVE_NODE_RPC_PORT` | `0`     | Optional direct host port for the raw archive node RPC. |

### Logging variables

| Variable               | Default             | Purpose                                      |
| ---------------------- | ------------------- | -------------------------------------------- |
| `ARGON_RPC_RUST_LOG`   | `info,subway=debug` | Rust log filter passed as `RUST_LOG`.        |
| `ARGON_RPC_LOG_FORMAT` | `compact`           | Subway log formatter passed as `LOG_FORMAT`. |

### Gateway variables

| Variable                                        | Default                      | Purpose                                                                                          |
| ----------------------------------------------- | ---------------------------- | ------------------------------------------------------------------------------------------------ |
| `ARGON_RPC_ENDPOINTS`                           | `["ws://archive-node:9944"]` | YAML/JSON list of upstream websocket endpoints. Order matters. Put the preferred endpoint first. |
| `ARGON_RPC_UPSTREAM_REQUEST_TIMEOUT_SECONDS`    | `20`                         | Per-request timeout for upstream RPC calls.                                                      |
| `ARGON_RPC_UPSTREAM_CONNECTION_TIMEOUT_SECONDS` | `5`                          | Timeout when opening an upstream websocket connection.                                           |
| `ARGON_RPC_UPSTREAM_RETRIES`                    | `1`                          | Number of retry attempts for upstream requests.                                                  |
| `ARGON_RPC_STALE_TIMEOUT_SECONDS`               | `180`                        | How long the substrate API extension tolerates stale head data.                                  |
| `ARGON_RPC_CACHE_TTL_SECONDS`                   | `60`                         | Default cache TTL for methods that use the shared cache.                                         |
| `ARGON_RPC_CACHE_SIZE`                          | `500`                        | Default cache entry limit for methods that use the shared cache.                                 |
| `ARGON_RPC_KEEP_ALIVE_SECONDS`                  | `60`                         | Subscription keepalive interval for merged subscriptions.                                        |
| `ARGON_RPC_MAX_CONNECTIONS`                     | `2000`                       | Maximum concurrent client connections accepted by the server.                                    |
| `ARGON_RPC_MAX_BATCH_SIZE`                      | `10`                         | Maximum JSON-RPC batch size accepted by the server.                                              |
| `ARGON_RPC_CONNECTION_BURST`                    | `20`                         | Burst limit for the connection-level rate limiter.                                               |
| `ARGON_RPC_CONNECTION_PERIOD_SECS`              | `1`                          | Window size for the connection-level rate limiter.                                               |
| `ARGON_RPC_IP_BURST`                            | `500`                        | Burst limit for the IP-level rate limiter.                                                       |
| `ARGON_RPC_IP_PERIOD_SECS`                      | `10`                         | Window size for the IP-level rate limiter.                                                       |
| `ARGON_RPC_USE_XFF`                             | `true`                       | Whether to trust `X-Forwarded-For` when applying IP rate limits.                                 |
| `ARGON_RPC_PROMETHEUS_LABEL`                    | `argon-rpc`                  | Label prefix exported by the Prometheus metrics endpoint.                                        |

## Endpoint examples

Single upstream:

```dotenv
ARGON_RPC_ENDPOINTS=["ws://archive-node:9944"]
```

Primary plus backup upstream:

```dotenv
ARGON_RPC_ENDPOINTS=["ws://archive-node:9944", "ws://backup-node:9944"]
```

Because the value is substituted directly into YAML, it needs to stay a valid YAML list literal. The
JSON-style array examples above work well in `.env` files.

## Health and metrics

- `GET /health` maps to `system_health`
- `GET /liveness` maps to `chain_getBlockHash`
- Prometheus metrics are exposed on container port `9616`

## OCI metadata

The image adds custom upstream provenance labels:

- `io.argonprotocol.upstream-subway.repository`: upstream Subway repository
- `io.argonprotocol.upstream-subway.ref`: pinned upstream Subway tag or ref

Published images already inherit the standard OCI labels and build metadata from the shared GitHub
Docker metadata workflow, and the shared publish template already emits a build provenance
attestation. The Dockerfile only adds the Argon-specific upstream Subway labels above.

## RPC surface

[`rpcs.yml`](./rpcs.yml) is the checked-in allowlist for exposed methods and their cache settings.
Right now it is intentionally scoped to the methods our local network and clients need, including:

- author methods for extrinsic submission
- common chain and state lookups
- payment fee queries
- selected `chainSpec_v1_*`, `transaction_v1_*`, and `archive_v1_body`

Two intentional exclusions are worth calling out:

- `rpc_methods` is not listed because Subway auto-registers it after loading the RPC config.
- `chainHead_v1_*` is omitted for now because Subway does not rewrite follow IDs for those
  downstream-follow subscriptions.

If you need to change the exposed RPC surface, edit [`rpcs.yml`](./rpcs.yml) and rebuild the image.

## Publishing status

The repo now has one workflow for this image at
[`publish-argon-rpc.yml`](../../.github/workflows/publish-argon-rpc.yml):

- pushes to `main` build the rolling edge image tags
- `workflow_dispatch` with `image_tag` publishes an immutable `vX.Y.Z-argon.N` tag from `main`

The manual tag path is intentionally narrow:

- it only accepts the full immutable Argon tag, such as `v0.1.2-argon.2`
- it does not allow overriding the pinned upstream Subway ref
- it always publishes current `main`
- it publishes `ghcr.io/argonprotocol/argon-rpc` only

The dev stack still builds this image locally from source, but the image itself no longer depends on
bind-mounted config files from the repo.
