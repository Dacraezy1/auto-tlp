# Releasing

Everything is automated by `.github/workflows/release.yml`.

## Cut a release

```console
scripts/bump-version.sh 0.2.0      # updates Cargo.toml, Cargo.lock and every packaging recipe
$EDITOR CHANGELOG.md               # add the new section
git commit -am "Release v0.2.0"
git tag v0.2.0
git push origin main v0.2.0
```

Pushing the tag runs, in order:

1. **Full CI** (`ci.yml`): rustfmt, clippy, tests on x86\_64 + aarch64 with stable and the MSRV, static musl build, TLP compatibility
   check, Nix build, source builds on five distros, the Arch `makepkg` build, and package install tests.
2. **Build** (x86\_64 and aarch64, natively): `.tar.gz`, `.deb`, `.rpm`, `.AppImage`, each installed/run again in clean containers.
3. **Arch package** built with `makepkg`.
4. **Publish**: source tarball, `SHA256SUMS`, build-provenance attestation, GitHub Release with generated notes.

The workflow refuses to publish if the tag does not match the version in `Cargo.toml`.
Running the workflow manually ("Run workflow") builds everything but does **not** publish.

## Release page has no files?

Assets are attached by the last job of the **Release** workflow, so the page stays empty until the whole run is
green. The first run takes about 25-40 minutes (full CI, then native x86\_64 and aarch64 builds).

1. Open **Actions → Release** and look for a run named after your tag.
   - *Running*: wait. The files appear when the last job ("Publish GitHub release") finishes.
   - *Failed*: open it, read the red step. From a terminal: `gh run list --workflow Release` then
     `gh run view <id> --log-failed`.
   - *No run at all*: the tag was probably not pushed from a commit that contains `release.yml`.
2. **Re-run for an existing tag** without touching git: Actions → Release → *Run workflow* → type the tag
   (for example `v0.1.0`). It rebuilds from that tag and attaches everything to the existing release.
   The tag must point at a commit that already has the fixes you want.
3. **Start clean** (delete the release and tag, then tag the current commit again):

   ```console
   gh release delete v0.1.0 --cleanup-tag --yes     # or delete the release in the web UI
   git tag -d v0.1.0
   git pull && git tag v0.1.0 && git push origin v0.1.0
   ```

If "Publish GitHub release" fails with *Resource not accessible by integration*, set
Settings → Actions → General → Workflow permissions to **Read and write**.
A *draft* release is not found by tag name; publish or delete it first.

## Release assets

| File | Purpose |
|---|---|
| `auto-tlp-<v>-<arch>-linux.tar.gz` | static binary + man page + completions + `Makefile` |
| `auto-tlp_<v>_<amd64\|arm64>.deb` | Debian / Ubuntu family |
| `auto-tlp-<v>-1.<arch>.rpm` | Fedora / RHEL / openSUSE family |
| `auto-tlp-<v>-<arch>.AppImage` | runs anywhere |
| `auto-tlp-<v>-1-x86_64.pkg.tar.zst` | Arch Linux |
| `auto-tlp-<v>-src.tar.gz` | source |
| `SHA256SUMS` | checksums of all of the above |

## After the release

Refresh the checksums in the distro recipes (they use placeholders in git):

```console
cd packaging/arch   && updpkgsums
cd packaging/alpine && abuild checksum
# Void:   xgensum -i auto-tlp
```

Then publish the recipes: push `packaging/arch/PKGBUILD` to the AUR as `auto-tlp`, and open PRs against
`void-packages`, Alpine `aports`, or a Gentoo overlay if you want them in official trees.

## Local dry run

```console
rustup target add x86_64-unknown-linux-musl
cargo install cargo-deb --version 2.12.1 --locked
cargo install cargo-generate-rpm --locked
scripts/build-dist.sh x86_64-unknown-linux-musl dist
scripts/smoke-test-packages.sh dist        # needs docker
```
