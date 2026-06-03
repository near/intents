# Ops

Runbook for deploying the outlayer worker to [Phala Cloud](https://cloud.phala.network).

## Configuration

Two env templates live in `ops/` — copy the relevant one and fill in the values:

- **Local** — `cp ops/env.local.example ops/.env` (just a dev seed; the image is
  built locally, so no registry credentials).
- **Production** — `cp ops/env.production.example .env` (repo root); fill in the
  GHCR pull credentials and signer seed, then deploy with `-e .env`. The image is
  **not** set here — it's pinned by digest in `ops/compose.yaml`.


## Development

Run the worker locally with Docker Compose. The compose files live in `ops/`, so
run from there (auto-merge needs both files in the working directory):

```sh
cp ops/env.local.example ops/.env   # first time only
cd ops
docker compose up --build
```


## Phala deployment

Install the [`phala` CLI](https://github.com/Phala-Network/phala-cloud/tree/main/cli)
(or run ad-hoc with `npx phala <command>`):

```sh
npm install -g phala
```

Authenticate via the browser device flow

```sh
phala login
```

## First deploy (creates a new CVM)

```sh
phala deploy \
    --name outlayer \
    --compose ops/compose.yaml \
    --vcpu 4 \
    --memory 4G \
    --disk-size 1G \
    --image dstack-0.5.9 \
    --kms phala
```

## Upgrade an existing CVM

Redeploy the current compose + env onto a running CVM (its `--cvm-id` is shown by
`phala cvms list` and in the web UI, prefixed with `app_`). Pass the env file with
`-e` so the updated values are applied:

```sh
phala deploy --cvm-id app_<CVM ID> --compose ./ops/compose.yaml -e .env
```

This ships new code only if the pinned image digest in `ops/compose.yaml` changed;
if you changed the worker, build & push it and bump that `@sha256:` digest first.

### Upgrading an app with active replicas

`phala deploy` targets a single `--cvm-id`; there is no CLI command to roll a new
compose out to a whole app at once. When an app has multiple active replicas you
must run the upgrade for each CVM individually. List them with:

```sh
phala cvms list
```

Replicas are named by appending a postfix to the base app name, so the base app
has the **shortest** name. Upgrade the longest-named CVMs (the replicas) **first**
and the shortest-named one (the base app) **last**.

## Create a replica

Scale an app out by replicating an existing CVM onto another node. The positional
argument is the **source** CVM to copy; `--node-id` is the target node (list
available nodes with `phala cvms list-node`). Pass `-e .env` so the replica gets
the same env:

```sh
phala cvms replicate app_<CVM ID> --node-id <node-id> -e .env
```

If the source app already has multiple live instances, you must say which
deployment to copy with `--compose-hash <hash>`:

```sh
phala cvms replicate app_<CVM ID> --node-id <node-id> --compose-hash <hash> -e .env
```

> `cvms replicate` is marked **unstable** in the CLI, and a fresh replica is the
> longest-named CVM — fold it into the replicas-first / base-app-last ordering used
> by upgrades and purges.

## Call the gRPC endpoint

The worker serves `outlayer.OutlayerService` (plus server reflection and
`grpc.health.v1`) on container port `50051`. The dstack gateway fronts it with a
public TLS endpoint.

The exact ingress hostname can be fetched from the Phala web UI, but it usually
follows `<appid>-<port>[g].dstack-pha-prod5.phala.network`, where the trailing
`g` marks the port as gRPC. Connect over TLS on `:443` — e.g.:

```sh
grpcurl <APP ID>-50051g.dstack-pha-prod5.phala.network:443 list
grpcurl <APP ID>-50051g.dstack-pha-prod5.phala.network:443 \
    grpc.health.v1.Health/Check
```


## Purge all CVMs

Lists `phala cvms delete` commands ordered so the shortest name (the base app) is
last — same ordering rule as upgrades:

```sh
phala cvms list --json \
    | jq -r '.items | sort_by(.cvmName | length) | reverse | .[] | "phala cvms delete \"\(.cvmName)\" --force"'
```

Review the output, then pipe to `sh` to run them in order:

```sh
phala cvms list --json \
    | jq -r '.items | sort_by(.cvmName | length) | reverse | .[] | "phala cvms delete \"\(.cvmName)\" --force"' \
    | sh
```

## Notes

- Phala CVMs run **`linux/amd64`** only. The published image must be built for
  that platform — on Apple Silicon, cross-build with
  `docker buildx build --platform linux/amd64 …` (a native arm64 image will fail
  to start on the CVM).
- **TEE attestation.** The whole `compose.yaml` is hashed into the CVM's
  attestation, so the image must be hardcoded **by digest**
  (`…/outlayer-worker@sha256:…`) — that's what binds the exact worker image to the
  attested identity. A tag wouldn't do: only the digest pins the image bytes.
