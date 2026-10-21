# Deploy sketch: ovillo.sophia-labs.com as a public gardend (NOT applied)

Mirrors the cow-tools deployment (`deploy/two-box/cow-tools/`) in Sophia's platform repository, which is not public. Nothing here has been installed or run, and nothing was written to the server.

| file | from cow-tools | change |
|---|---|---|
| `systemd/ovillo.service` | `cow-tools.service` | loopback **7692** (cow-tools is 7690); user `ovillo`; no services, no ONNX runtime/fastembed; caps 0.5 vCPU / 1 GiB high / 2 GiB max |
| `etc/public.json` | `etc/public.json` | `graphs:["ovillo"]`, `siteDir` the release's `site/`; no `services`, `accounts`, `visitorFolders` (nobody writes) |
| `caddy/ovillo-block.caddy` | `caddy/routes.caddy` + the cow-tools block | **replaces** the static ovillo block in `deploy/two-box/caddy/Caddyfile` (see below) |
| `release.sh` | `stage-release.sh` | builds graph + site, runs the test, ships `gardend` + `site/` + `ovillo.nt`; the profile is rebuilt by `gardend load` |

Still to write when it is real (as cow-tools has them): the ansible role `ovillo` (user, dirs, unit, owner key in `/etc/ovillo/owner.env`),
`ovillo-release` / `ovillo-profile` on the box, `deploy-ovillo.sh` retired (the static `r4` release stays in
`/srv/sophia/static/ovillo/releases/` as the rollback: point the old block back).

## What changes from the static Caddy block

- **Not just files any more**: `/mcp`, `/graphs`, `/health`, `/self-host.json` go to gardend on loopback; everything else is Caddy's `file_server`
  on the release's `site/`, the same directory as `siteDir`. Why not let gardend serve the files: gardend 0.1.4 answers `Range` with 200 and the whole
  file (observed, `curl -H 'Range: bytes=0-99'`), which breaks seeking in `<audio>` (Safari above all), and it sends `.mus` as `application/octet-stream`.
- **CSP unchanged** (`'wasm-unsafe-eval'`, `worker-src 'self' blob:`, `media-src 'self' blob:`, `connect-src 'self'`). The app reads the graph at its own
  origin's `/mcp`, so no new origin enters it.
- **IRIs**: the graph's `ovl:audio` / `ovl:peaks` / `ovl:energyTrace` / `ovl:receipt` are absolute `https://ovillo.sophia-labs.com/...`. They resolve on the
  box as is; on a local gardend (port 7792) a client takes the pathname and fetches it from its own origin.
- **Rate limit** 240/min per address (the app fetches peaks and energy per piece); static files are not counted by gardend anyway when it serves them, and
  Caddy-served files never reach it.
- **Switch**: `/srv/sophia/ovillo/switches/public.off` (a file Caddy stats) answers 503 "resting", as cow-tools' does.
- **Profile**: graph `ovillo` created once (`create_graph`), then `gardend load --graph ovillo ovillo.nt` while gardend is stopped. The load is atomic
  and idempotent (a set of quads); to replace data, build a new profile and swap, as `cow-tools-profile install` does.
- **Backups**: the graph is rebuilt from the repo and the renders; only the media (`site/media`) and `ovillo.nt` of a release need keeping. A restic tag
  `ovillo` like cow-tools' is optional.
