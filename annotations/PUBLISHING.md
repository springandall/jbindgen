# JBindgen Annotations - Publishing to Maven Central

This document describes how to publish the `jbindgen-annotations` library to Maven Central using Gradle.

## Publishing Strategy

- **Release versions** (e.g., `0.1.0`, `0.2.0`): Published via **GitHub
  Actions** by pushing an `annotations-vX.Y.Z` tag for the `main` branch.
- **Snapshots** (e.g., `0.1.0-SNAPSHOT`): Can be published **locally** by developers
- **Release candidates** (e.g., `0.1.0-RC1`): Can be published **locally** for testing

## Prerequisites

### 1. Maven Central Account for Local Publishing

If you need to be able to publish snapshots or release candidates from your
local development environment, (as opposed to relying on GitHub Actions), you
must have access to the `io.github.jni-rs` groupId on Maven Central.

1. [Create an account](https://central.sonatype.org/register/central-portal/#create-an-account)
   for Maven Central (if you don't have one)
2. Open a Github issue to request for an existing maintainer (@rib) to add you
   to the `io.github.jni-rs` groupId
3. Generate a user token:
   - Click top-right profile picture, then "View User Tokens"
   - Click "Generate User Token"

### 2. GPG Key for Local Signing

Each developer should have their own GPG key for signing locally published artifacts.

```bash
# Generate a new GPG key
gpg --full-generate-key

# When prompted:
# - Key type: (1) RSA and RSA
# - Key size: 4096
# - Expiration: 2 years (recommended)
# - Real name: Your Name
# - Email: your-email@example.com
# - Passphrase: Choose a strong passphrase

# List your keys to get the key ID
gpg --list-secret-keys --keyid-format=long

# The output shows something like:
# sec   rsa4096/ABCD1234EFGH5678 2026-02-10 [SC]
# The key ID is: ABCD1234EFGH5678

# Upload your public key to keyservers
gpg --keyserver keys.openpgp.org --send-keys ABCD1234EFGH5678
gpg --keyserver keyserver.ubuntu.com --send-keys ABCD1234EFGH5678
```

## Local Development Setup

### Configure Gradle Properties

Create or edit `~/.gradle/gradle.properties`:

```properties
# Maven Central Credentials
jbindgen.centralUsername=your-token-username
jbindgen.centralToken=your-token-password

# GPG Signing - using GPG agent (recommended, uses default key and prompts for passphrase)
signing.gnupg.executable=gpg

# Alternative: Store passphrase (less secure)
# signing.keyId=ABCD1234EFGH5678
# signing.password=your-gpg-passphrase
# signing.secretKeyRingFile=/home/you/.gnupg/secring.gpg
```

Set secure permissions:

```bash
chmod 600 ~/.gradle/gradle.properties
```

**Recommendation**: Use `signing.gnupg.executable=gpg` to use the GPG agent,
which will prompt you for your passphrase when signing. This avoids storing your
passphrase in plaintext.

## Version Management

The project uses a versioning scheme with separate base version and suffix:

- **Base version**: Defined in `build.gradle` (e.g., `0.1.0`)
- **Version suffix**: Configurable via command-line, defaults to `-SNAPSHOT`
- **Full version**: `baseVersion + versionSuffix`

### Command-Line Version Overrides

You can override the version suffix using Gradle properties:

```bash
# Use default snapshot version (0.1.0-SNAPSHOT)
./gradlew build

# Override suffix for a release candidate
./gradlew publish -PversionSuffix=-RC1
# Result: 0.1.0-RC1

# Another release candidate
./gradlew publish -PversionSuffix=-RC2
# Result: 0.1.0-RC2

# Full release (requires safety flag)
./gradlew publish -PversionSuffix= -PallowFullRelease=true
# Result: 0.1.0
```

**Note:** The base version is set in `build.gradle` and must be updated there
when bumping versions.

### Release Safety Check

To prevent accidental full releases, publishing without a version suffix
requires the `-PallowFullRelease=true` flag. Attempting to publish without this
flag will fail with an error:

```bash
# This will FAIL with an error
./gradlew publish -PversionSuffix=

# This will succeed
./gradlew publish -PversionSuffix= -PallowFullRelease=true
```

## Publishing from Local Development

### Test Build and Signing

```bash
cd annotations

# Clean build (using default SNAPSHOT version)
./gradlew clean build

# Test signing (creates .asc signature files in build/libs/)
./gradlew signMavenPublication

# Verify generated artifacts in build/libs/:
# - jbindgen-annotations-0.1.0-SNAPSHOT.jar
# - jbindgen-annotations-0.1.0-SNAPSHOT.jar.asc
# - jbindgen-annotations-0.1.0-SNAPSHOT-sources.jar
# - jbindgen-annotations-0.1.0-SNAPSHOT-sources.jar.asc
# - jbindgen-annotations-0.1.0-SNAPSHOT-javadoc.jar
# - jbindgen-annotations-0.1.0-SNAPSHOT-javadoc.jar.asc
```

### Publish to Local Maven Repository

```bash
# Publish to ~/.m2/repository/ for local testing
./gradlew publishToMavenLocal

# Verify in ~/.m2/repository/io/github/jni-rs/jbindgen-annotations/
```

### Publish Snapshot

Snapshots are mutable and useful for development/testing.

```bash
# Publish snapshot with default version (0.1.0-SNAPSHOT)
./gradlew publish

# Published to: https://central.sonatype.com/repository/maven-snapshots/
#   io/github/jni-rs/jbindgen-annotations/0.1.0-SNAPSHOT/maven-metadata.xml
```

**Notes:**
- You can overwrite snapshots (they're mutable)
- Signing is optional but recommended
- The default suffix is `-SNAPSHOT` (no need to specify `-PversionSuffix=-SNAPSHOT`)
- To change the version number, update `baseVersion` in `build.gradle`

### Publish Release Candidate

Release candidates are immutable releases for testing before final release.

```bash
# Publish release candidate (no build.gradle changes needed)
./gradlew publish -PversionSuffix=-RC1
# Result: 0.1.0-RC1

# Subsequent RC for the same base version
./gradlew publish -PversionSuffix=-RC2
# Result: 0.1.0-RC2
```

**Notes:**
- Signing is REQUIRED for release candidates
- Release candidates are immutable (cannot be overwritten)
- To change the version number, update `baseVersion` in `build.gradle`

### Publishing a Release

Production releases should be published through GitHub Actions for consistency
and audit trail.

```bash
# Create an annotations library release branch
git checkout -b release-annotations-0.2.0

# Update the base version in build.gradle
# Change: def baseVersion = '0.1.0'
# To:     def baseVersion = '0.2.0'

# Commit the version change
git add annotations/build.gradle
git commit -m "Release annotations v0.2.0"
git push origin HEAD
gh pr create --title "Release annotations v0.2.0"

# Merge the PR on Github into the `main` branch

# Create and push a tag (triggers GitHub Actions)
git fetch origin
git tag annotations-v0.2.0 origin/main
git push origin annotations-v0.2.0

# The GitHub Action will automatically:
#    - Build the library
#    - Sign artifacts with the organization's GPG key
#    - Publish to Maven Central
```

**Note:** The annotations library should always be released before the main
jbindgen library, since the main library depends on the annotations.

### Setup GitHub Organization Secrets

For reference, the GPG Key for Github Actions was generated like this:

```bash
# Generate a dedicated key for automated releases
gpg --batch --gen-key <<EOF
Key-Type: RSA
Key-Length: 4096
Subkey-Type: RSA
Subkey-Length: 4096
Name-Real: JNI-RS JBindgen CI
Name-Email: 41898282+github-actions[bot]@users.noreply.github.com
Expire-Date: 0
EOF

# Upload public key to keyservers
gpg --keyserver keys.openpgp.org --send-keys <KEY_ID>
gpg --keyserver keyserver.ubuntu.com --send-keys <KEY_ID>

# Export private key (base64 encoded for MAVEN_GPG_PRIVATE_KEY)
gpg --export-secret-keys --armor <KEY_ID> | base64 -w0 > gpg-key.txt

# The passphrase will be needed for MAVEN_GPG_PASSPHRASE
```