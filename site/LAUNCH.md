# Launch on the home server

The site is static. The intended route is the existing Cloudflare Tunnel to a
localhost-only Caddy origin. The public hostname is chosen separately; this
guide does not create a tunnel, change DNS or edit existing public routes.

## Prepare a release

From the committed checkout:

```sh
node scripts/build-site.mjs --check
node scripts/check-site.mjs
node --test scripts/site.test.mjs scripts/check-public-site.test.mjs
```

Copy the contents of `site/` and `deploy/Caddyfile` to the server, for example
under `/srv/crosscheck/site/` and `/srv/crosscheck/deploy/Caddyfile`. Use a
versioned release directory and keep the previous release until the public
check passes. If using a `current` symlink, set `CROSSCHECK_SITE_ROOT` to its
site directory. Publish only `site/`, never the repository root or `.canvas/`.

## Run the origin

Caddy 2 must be installed. Validate first, then run:

```sh
CROSSCHECK_SITE_ROOT=/srv/crosscheck/site caddy validate --config /srv/crosscheck/deploy/Caddyfile --adapter caddyfile
CROSSCHECK_SITE_ROOT=/srv/crosscheck/site caddy run --config /srv/crosscheck/deploy/Caddyfile --adapter caddyfile
```

The config binds HTTP port 8787 to `127.0.0.1`, disables the admin listener and
autosave persistence so this origin does not overwrite another Caddy autosave,
and serves the site without directory browsing. It returns 404 for the site's
maintenance Markdown files. Cloudflare supplies the public HTTPS endpoint.
If port 8787 is occupied, choose a free port in both Caddy and the tunnel route.

For a Linux host with systemd, `deploy/crosscheck-site.service.example` is an
optional service unit. Adjust the Caddy binary path, release path and service
user before installing it. The user needs read access to the files. Check the
origin with `curl http://127.0.0.1:8787/` before adding the public route.

## Add the existing tunnel route

For a dashboard-managed tunnel, add a published application route for the chosen
hostname with service type HTTP and URL `127.0.0.1:8787`. Retain all existing
application routes. For a locally managed tunnel, insert the equivalent ingress
rule before its final catch-all, leaving the existing tunnel ID, credentials
and other ingress rules intact.

This localhost address assumes cloudflared runs on the host. If cloudflared
runs in a container, its `127.0.0.1` is the container itself. Use your existing
shared-network origin pattern, with a reachable Caddy address; do not point a
container at this host-loopback-only listener and expect it to work.

The study is meant for public reading. Check the route in a signed-out browser
so an Access login or challenge does not prevent hiring readers from opening it.
Use the same method already used for the server's other public apps.

References: [Cloudflare published applications](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/routing-to-tunnel/),
[Caddy static files](https://caddyserver.com/docs/caddyfile/directives/file_server),
[Caddy loopback binding](https://caddyserver.com/docs/caddyfile/directives/bind).

## Verify the public release

On a machine with this same release checked out, run the following command,
replacing `https://YOUR-HOSTNAME/` with the chosen public URL:

```sh
node scripts/check-public-site.mjs https://YOUR-HOSTNAME/
```

The command checks page identity, contribution/replay sections, the exact JSON
publication snapshot and the deployed CSS, script and fonts. It fails if the
URL serves the old worktree or a different release. Run it over HTTPS for the
public launch; HTTP is supported only for local origin testing.

Open the home page, `/everest.html#contribution`, `/everest.html#witness` and
`/everest.html#replay` on desktop and phone. Download the record and follow its
pinned links. Record the checked release commit and public URL in your local
launch notes. A local preview or a prepared config is not a completed launch.

## Connect the publications

Once the public check passes, use the stable `/everest.html` URL in applications
and outreach. The shorter `#contribution` link is useful for a hiring reader;
`#witness` opens the concrete finding and `#replay` opens the reproduction path.
An optional update to the published essay can link to the study. Draft language
is in `docs/publication/crosscheck-story.md`; it is not sent or published by
building the site.

If the public check fails after a release, restore the previous release directory,
repeat the check, and inspect the tunnel/origin before sharing the new URL.
