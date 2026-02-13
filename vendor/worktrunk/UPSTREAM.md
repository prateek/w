# Vendored upstream: Worktrunk

This directory is vendored from upstream Worktrunk via `git subtree`.

Upstream:

- Repo: `https://github.com/max-sixty/worktrunk`
- Tag: `v0.23.2`
- Commit: `eb9023a030ccee5b3a1a3a03086ad25472f85eba`

## Sync upstream into this repo

Update to a newer tag/commit:

```bash
git subtree pull --prefix vendor/worktrunk https://github.com/max-sixty/worktrunk.git <ref>
```

## Upstream patches from this repo

Keep vendored-only changes in clean commits that touch only `vendor/worktrunk/`, then:

```bash
git subtree split --prefix vendor/worktrunk -b worktrunk-upstreamable
```

