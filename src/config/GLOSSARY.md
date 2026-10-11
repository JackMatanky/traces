# Config

Project configuration discovery, TOML parsing, workspace trust verification,
and configuration tracking.

## Language

### Configuration Files

#### Config File

A TOML configuration file at the local (`.traces/config.toml`) or global
(`~/.config/traces/config.toml`) level defining template, schema, and metadata
settings.
*Avoid*: settings file, preferences

#### Configuration Scope

The two-tier hierarchy where global defaults are overridden by local project
configuration.
*Avoid*: layered config, cascaded settings

### Trust & Security

#### Trust Verification

The security check performed before loading local configuration or executing
templates, rejecting untrusted or altered project roots.
*Avoid*: authorization check, security scan

#### Trust Status

The trust state of a workspace root: `Trusted` (known root with matching
digest), `Untrusted` (unknown root), `Stale` (configuration modified since trust
was granted), or `MissingBaseline` (trusted root with no recorded digest for the
config file).
*Avoid*: trust level, security state

#### Companion Hash

The stored cryptographic digest of a `.traces/config.toml` file used to detect
out-of-band configuration changes.
*Avoid*: baseline hash, signature, checksum

#### Tracked Config Path

The path of a local `.traces/config.toml` file recorded when configuration
loading sees it or when trust is granted to it. Tracking a config path is
independent of whether its workspace is trusted, but every trusted root that has
a local config has that config path tracked: a grant records the path before it
records trust, and fails if it cannot. Revoking trust leaves the path tracked.
*Avoid*: config history, audit store

#### Trust Store

The persistent record of trusted workspace roots and their Companion Hashes.
*Avoid*: trust database, allowlist

#### Tracker Store

The best-effort record of Tracked Config Paths. Loading records into it without
failing; granting trust records into it and fails if it cannot.
*Avoid*: seen-config registry

### Discovery

#### Config Locator

Finds config files without reading their TOML, using two ambient inputs: the
ceiling directories an upward walk stops at, and the global config directory. It
knows nothing about trust or tracking.
*Avoid*: config finder, config walker

#### Template Directory

Any directory inside the workspace, named by `[templates] directory` relative to
the workspace root. Absolute paths, `..` components, and symlinks leaving the
workspace are rejected. Defaults to `.traces/templates`; `traces init` creates
the directory you choose.
*Avoid*: template path, template root

### Fingerprints

#### Config Fingerprint

A deterministic digest over every resolved value of a loaded configuration,
built with the length-prefixed `Blake3Hasher`. Reserved for index cache
invalidation; its framing and field order are pinned by a test because stored
digests must survive releases.
*Avoid*: config hash, checksum

### Canonical Metadata

#### Metadata Roles

Canonical frontmatter keys configured under `[frontmatter]` mapping
project-specific names for title, aliases, tags, creation date, and modification
date.
*Avoid*: field mapping, canonical attributes

### Task Recognition

#### Task Configuration

The `[tasks]` table configuring task recognition: the mapping of checkbox status
symbols to Task Status workflow states, and the tag filter deciding which tagged
items are tasks. Configured statuses augment defaults and override a default
that shares their symbol. For statuses and tag filters independently, a
non-empty local list replaces the global list; an absent or empty local list
falls back to the global list.
*Avoid*: status settings, task options
