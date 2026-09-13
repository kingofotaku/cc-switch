# Model-Aware Runtime Maintenance

This branch integrates the local Codex compatibility layer with CCS v3.20.3.
It keeps only behavior that upstream does not yet provide and adopts the
official implementation whenever v3.20.3 already covers the same feature. It
is intentionally maintained as a source branch rather than as a permanent
binary fork.

## Architecture

The project has two cooperating layers:

```text
official model documentation
        |
Codex capability profiles (external catalog-repair project)
        |
cc-switch provider modelCatalog.models[*]
        |
CCS Codex request resolver
        |
model-specific transport mapping
        |
upstream provider API
```

The external catalog-repair project owns model discovery, family matching,
input/tool capability metadata, and verified reasoning levels. CCS consumes
that metadata, preserves it through the provider UI, and converts the selected
effort to the transport field required by the selected provider. The
provider-level CCS setting remains a fallback for legacy rows that have no
model-level metadata.

CCS v3.20.3 is authoritative for OAuth crash-recovery preservation, catalog
`reasoningLevels` / `defaultReasoningLevel`, and DeepSeek V4 text-only input
modality inference. The local branch does not duplicate those implementations.

Context-window values are deliberately outside this feature. Existing
`contextWindow`, `maxContextWindow`, and
`effectiveContextWindowPercent` values remain operator-owned and are not
generated or overwritten by the runtime patch.

## Branch Layout

- Upstream remote: `upstream` (`farion1231/cc-switch`)
- Personal fork remote: `origin` (`kingofotaku/cc-switch`)
- Working branch: `codex/model-aware-catalog-v3.20.3`
- Capability metadata remains generated outside CCS by the Codex catalog repair
  pipeline; this repository only teaches CCS how to consume model-level data.

## Changed Files

- `src-tauri/src/codex_config.rs`: retains the stable custom-provider tool
  surface for GPT-5.6 and GPT-6 catalog entries and strips backend-only tool
  mode fields from non-OpenAI custom/vendor catalog entries.
- `src-tauri/src/proxy/providers/codex.rs`: resolves model-level reasoning
  metadata first, accepts common model-id aliases, and preserves the existing
  provider-level fallback.
- `src-tauri/src/proxy/providers/transform_codex_chat.rs`: maps the
  model-specific `thinking_level` transport used by Gemini-style providers
  while retaining the official Zen effort path.
- `src-tauri/src/proxy/endpoint_failover.rs` and forwarder/router integration:
  rotates a multi-endpoint provider after three failed requests, persists the
  selected endpoint, and then preserves the existing provider-level failover
  behavior.
- `src-tauri/src/database/dao/providers.rs`: persists endpoint selection with
  compare-and-swap semantics and hydrates custom endpoints for routing.
- `src/types.ts`: keeps model-level capability metadata in the Codex catalog
  type instead of dropping it during serialization, including official
  reasoning-level fields and local `codexChatReasoning` transport metadata.
- `src/components/providers/forms/ProviderForm.tsx`: preserves hidden
  capability metadata when the provider form saves a catalog.
- `src/components/providers/forms/hooks/useCodexConfigState.ts`: loads and
  retains model-level metadata across UI state transitions.
- Endpoint speed-test/provider UI files expose the endpoint-failover switch
  only where multiple endpoints exist.
- `tests/components/ProviderForm.codexCatalog.test.ts`: frontend round-trip
  regression coverage.
- The first model-aware commit is the reproducible source delta from the
  recorded upstream base. Generate a temporary patch with
  `git format-patch --stdout origin/main..HEAD` when a downstream maintainer
  needs a portable patch; do not commit generated patch files with structural
  whitespace into this repository.

## Resolution Rules

1. Match the selected catalog model by `model`, `modelId`, `model_id`, `slug`,
   `id`, or `name`.
2. If the catalog row contains explicit `codexChatReasoning`, use it.
3. Otherwise use the provider-level `codexChatReasoning` fallback.
4. If neither exists, retain the existing conservative inference path.
5. Never invent `xhigh`, `max`, image input, audio input, or tool support for an
   unknown model.

An exact model rule is authoritative over stale provider-imported capability
fields for modality and reasoning metadata. This lets a documented model update
(for example, the current DeepSeek Flash vision and reasoning levels) repair a
provider row without changing its operator-owned `contextWindow`,
`maxContextWindow`, or `effectiveContextWindowPercent` values. Generic family
rules remain lower priority, and explicit user context settings are never
overwritten by capability projection.

The current DeepSeek mapping follows the official API documentation:
`deepseek-flash` and the legacy `deepseek-v4-flash` alias accept images, while
`deepseek-v4-pro` is text-only. Both expose `low`, `high`, and optional `max`
reasoning effort with `high` as the default.

## Codex Tool-Surface Compatibility

Codex `0.154.0-alpha.6.2` and upstream `main` commit `1715e55` still mark
GPT-5.6 and GPT-6 catalog entries as `tool_mode = code_mode_only` with
`use_responses_lite = true` and `comp_hash = 3000`. That backend-internal
surface does not expose the local shell and `apply_patch` tools correctly when
the model is reached through a third-party Codex provider.

The compatibility projection therefore continues to emit the stable `2911`
surface (`use_responses_lite = false`) and removes `tool_mode` and
`multi_agent_version` for GPT-5.6 and GPT-6 over proxy-chat providers. Do not
remove this projection solely because Codex has been updated; first re-check
the bundled model metadata for the installed Codex build and add a failing
regression test before deleting it.

The catalog-repair layer must therefore encode a model with only the effort
levels verified for that family. A model that supports only `high` must not
inherit `xhigh` merely because another model from the same provider supports
it.

## Updating From CCS

Run these steps after a new upstream CCS release or a substantial upstream
change. Do not replace the installed executable before the source and tests
pass.

```powershell
git remote add upstream https://github.com/farion1231/cc-switch.git
git fetch upstream --tags
git switch codex/model-aware-catalog-v3.20.3
git merge --no-ff v3.20.3
```

Before applying a future merge, inspect the upstream diff in Codex provider
resolution, Chat Completions transformation, catalog types, provider-form
serialization, endpoint routing, and OAuth recovery. A conflict in any of
these files is a behavioral conflict, not a cosmetic one. If upstream now
implements a local feature, delete the local duplicate and retain the official
implementation plus focused compatibility tests.

If the merge conflicts in the Codex provider or transform files, resolve the
conflict deliberately and keep the model-level precedence rule. Do not use an
automatic conflict resolution that silently restores provider-only reasoning.

Then run the repository checks:

```powershell
pnpm typecheck
pnpm format:check
pnpm test:unit
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

The Windows local build used for the Codex installation must be a Tauri
production build. A bare `cargo build --release` leaves the WebView on
`build.devUrl` (`http://localhost:3000`) and produces a backend that works while
the desktop UI shows `ERR_CONNECTION_REFUSED`.

Build the renderer and then use the repository-local Tauri CLI:

```powershell
& .\node_modules\.bin\vite.cmd build
& .\node_modules\.bin\tauri.cmd build --no-bundle --ci `
    --config .\src-tauri\tauri.model-aware-build.conf.json
```

On this Windows host, place the Rust toolchain bin directory first in `PATH`
and set `CARGO`, `RUSTC`, `CARGO_HOME`, and `RUSTUP_HOME` explicitly when the
Tauri CLI reports that Cargo is missing. A direct `cargo metadata` success is
not sufficient because the Tauri CLI launches Cargo in a child environment.

Do not use `cargo build --release` as the distributable build. A production
artifact can still contain the literal development URL because Tauri compiles
the complete build configuration into the executable, so a plain string scan
for `localhost:3000` is not a sufficient gate. Instead verify all of the
following:

- the release fingerprint enables Tauri's `custom-protocol` feature;
- a filename from the freshly generated Vite `dist/assets` set is embedded in
  the executable;
- the artifact version and size are consistent with the production baseline;
- a cold runtime smoke test does not request `localhost:3000` or
  `127.0.0.1:3000`.

The renderer must come from `build.frontendDist`; the development URL is never
an acceptable runtime dependency.

The small override only disables `beforeBuildCommand` because the renderer was
already built explicitly. Passing inline JSON through the Windows `.cmd`
launcher is intentionally avoided because its quoting rules can corrupt the
configuration argument.

The resulting `cc-switch.exe` is a local artifact and must not be committed to
this public repository. Record its version and SHA256 in the private installer
manifest used by the local Codex skill.

The source delta can also be checked against a clean tree with:

```powershell
& .\scripts\verify-model-aware-source.ps1
```

This check is read-only. It verifies the base relationship, working-tree
state, manifest/package version agreement, changed-file list, and whitespace
errors. The fixed base commit is read from
`runtime/model-aware-runtime-manifest.json`, so a later rolling `origin/main`
does not invalidate a release build. The check does not touch CCS, Codex, the
provider database, or any local credentials.

## Safety Gates

The local installer must refuse to apply when any of these conditions is true:

- CCS is still running.
- The artifact version does not match the intended CCS version.
- The artifact hash does not match the freshly built artifact.
- The source branch was rebased but the required tests have not passed.
- The target executable is newer than the artifact.

When a gate fails, keep the official/newer CCS executable in place and use the
catalog-only path. Catalog projection and context-window preservation do not
require replacing the CCS executable.

## Rollout And Rollback

1. Close CCS completely and confirm no process owns the target executable.
2. Back up the current executable and write a manifest containing the old
   version and SHA256.
3. Install only the artifact built from the tested branch commit.
4. Start CCS and run a no-secret local catalog/request conversion smoke test.
5. Verify one model from each relevant family before trying a live upstream
   request.
6. If startup, catalog rendering, or request conversion regresses, close CCS
   and restore the manifest-selected backup. Do not edit the provider database
   as a first response to a runtime mismatch.

The local Codex skill owns the Windows backup and rollback operation. This
public branch does not know the local executable path and does not perform the
replacement itself.

## Handoff Checklist

Before handing this work to another maintainer, record:

- upstream commit and package version used as the base;
- branch commit containing the model-aware runtime change;
- exact test commands and their results;
- local artifact version and SHA256 in the private installer manifest;
- whether the runtime was installed or only built;
- whether live catalog projection and live request verification were performed;
- any unresolved upstream conflicts or model-family limitations.

Never infer live deployment from a green source build. The public branch, local
binary, installed CCS process, and Codex catalog are four separate states.

## What Must Never Enter This Repository

Do not commit `cc-switch.db`, `config.toml`, `models_cache.json`, provider
catalog exports, logs, backups, cookies, `.env` files, API keys, bearer tokens,
OAuth credentials, or any generated file copied from the local installation.

Before pushing, review:

```powershell
git status --short
git diff --check
git diff --stat
```

Also inspect the added diff lines for credential-shaped values. Test fixtures
may use obvious placeholders, but never copy a real local value into a test.
