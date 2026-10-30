# Make the Crosscheck work legible to hiring readers

## Intent

The user selected evidence approach v3 on 2026-10-28. The latest Canvas document,
`nuanced/career-desires.md` v69, asks the website, essay and EVerest case study to
tell one story about real results, for applications and specific side-door
conversations. The public site must remain useful as a developer-tool site.

The site already presents the scientific record. It needs a concise account of
Harry Nicholls's contribution, the role of AI assistance and deterministic
checks, the key design decisions, and a path to discussing related work.

## Outcome and constraints

- Preserve the selected evidence-led design and existing pinned scientific data.
- Explain contribution without implying hand-authorship of agent-generated
  artifacts or a machine proof of the production implementation.
- Prepare application and outreach drafts; do not send messages or invent
  recipients, company needs, endorsements or broader production impact.
- Keep compensation, work authorization and detailed job filters in local
  `.canvas/` preparation notes, not in the public site or Git commits.
- Prepare deployment to the user's home server behind an existing Cloudflare
  Tunnel, with a Caddy origin on localhost and a public-route smoke check.
- The hostname is pending the user's answer. Deployment itself remains a
  separate user-operated action; no credentials, tunnel edits or publishing.

## Open decisions

The public hostname is not yet specified. All site navigation remains relative;
deployment instructions accept the chosen URL at check time. This does not
block implementation or local verification. The public launch cannot be claimed
until that destination has been routed and checked.

## Acceptance and plan

See [spec](2026-10-28-career-evidence-spec.md) and
[plan](2026-10-28-career-evidence-plan.md). Tier 2; no protected surfaces.
