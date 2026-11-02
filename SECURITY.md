# Security policy

auto-tlp only reads system files unless you pass `--output` or `--install`. `--install` writes one file under
`/etc/tlp.d/` and needs root.

If you find a security problem, please **do not open a public issue**. Use GitHub's
["Report a vulnerability"](https://github.com/Dacraezy1/auto-tlp/security/advisories/new) form on this repository.
Release artifacts carry GitHub build-provenance attestations and SHA-256 checksums
(`gh attestation verify <file> --repo Dacraezy1/auto-tlp`).
