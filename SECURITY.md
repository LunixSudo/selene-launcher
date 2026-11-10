# Security policy

## Supported versions

Only the latest release gets security fixes.

## Reporting a vulnerability

Please do not open a public issue. Report it privately through
[GitHub security advisories](https://github.com/LunixSudo/selene-launcher/security/advisories/new)
and include the steps to reproduce it and the launcher version.

You will get an answer within a week. Once a fix is released, the advisory is published
with credit to the reporter unless they ask otherwise.

## Checking a download

Official builds come only from this repository's
[releases](https://github.com/LunixSudo/selene-launcher/releases). Each release carries:

- `SHA256SUMS.txt` with the checksum of every file. On Linux, `sha256sum -c SHA256SUMS.txt`
  in the download folder compares them; on Windows, `Get-FileHash <file>` in PowerShell
  prints the checksum to compare by eye.
- A signed statement that the file was built by this repository's
  [Build](.github/workflows/build.yml) workflow from the tagged commit. With the
  [GitHub CLI](https://cli.github.com/):
  `gh attestation verify <file> --repo LunixSudo/selene-launcher`.
- An `.spdx.json` list of every library the build depends on.

The Windows installer has no code signature yet, so SmartScreen may warn about it.
