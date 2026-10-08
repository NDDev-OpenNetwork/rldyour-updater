# Publishing an approved catalogue

Catalogue promotion is separate from daily device updates. Use a reviewed clean
bootstrap checkout and an existing owner-held Ed25519 PKCS#8 PEM signing key. Never
commit or copy the private key to a workstation or CI secret as part of this flow.

    rldyour-updater catalog-build --bootstrap /absolute/bootstrap --sequence 2 --days 30 --output /absolute/payload.json --debs /absolute/reviewed-debs.json
    rldyour-updater catalog-sign --payload /absolute/payload.json --private-key /absolute/private/key.pem --output /absolute/stable.json
    rldyour-updater catalog-verify --catalog /absolute/stable.json --public-key <reviewed-public-key-hex>

Review exact source identity, every artifact source/hash and the sequence before
signing. Increment sequence even when renewing unchanged target metadata. Publish
stable.json through the repository's ordinary pull request flow to catalog/stable.json.
Clients provision that fixed HTTPS URL and key locally; their daily schedule
consumes it automatically without changing developer checkouts.

The signer command emits only public key material. A catalogue must be renewed
within its declared lifetime. No online key is installed on the clients; no
unsigned fallback, self-signed response or automatic trust-reset is supported.

Standalone Debian records use package, version, url, bytes and sha256. Mutable
latest URLs are not approved artifact identities. Do not add compatibility-sensitive
GPU/Python/model changes without a separately tested native contract.

Updater self-updates use an optional updater_binaries table in the signed payload:
platform (linux/x86_64 or macos/arm64), stable version, explicit release URL,
length and SHA256. Pass --updater-binaries to catalog-build to include reviewed
records. A higher version is downloaded, verified and atomically published only
at the managed installation path; the current run and all other apps stay alive.
The source-installer bootstrap trust check remains a separate operator step.
