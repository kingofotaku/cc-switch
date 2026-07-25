# Grok Build and Composer capability rules

This branch tracks the catalog-repair capability profile in
`runtime/codex-model-capability-profiles.json`. The file contains no provider
credentials and is safe to keep in the public fork.

## Reasoning mapping

| Model IDs | Codex UI levels | Request transport |
| --- | --- | --- |
| `grok-build`, `grok-build-latest` | `low`, `medium`, `high` | `reasoning_effort` |
| `grok-build-0.1` | fixed `high` | none; do not send an unsupported effort field |
| `composer-2.5`, `grok-composer`, `grok-composer-2.5-fast` | fixed `none` | none |

The fixed `none` entry for Composer is deliberate. If the catalog omits
`supported_reasoning_levels`, Codex Desktop falls back to a synthetic
`medium` option even though the model is non-reasoning.

## Evidence

- xAI states that Grok 4.5 is the default model in Grok Build, and its API
  reasoning levels are `low`, `medium`, and `high`:
  `https://x.ai/news/grok-4-5` and
  `https://docs.x.ai/developers/model-capabilities/text/reasoning`.
- The Grok Build 0.1 model page marks the model as reasoning-capable, while
  xAI's model matrix does not mark its reasoning as configurable:
  `https://docs.x.ai/developers/models/grok-build-0.1` and
  `https://docs.x.ai/developers/models`.
- xAI's Composer 2.5 announcement identifies the current Composer model. The
  official Grok Build client consumes server-provided
  `supports_reasoning_effort` and `reasoning_efforts` metadata. The public
  xAI OAuth adapter release records Composer 2.5 Fast as non-reasoning:
  `https://x.ai/news/composer-2-5`,
  `https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-shell/src/remote/client.rs`,
  and `https://github.com/nicepkg/pi-xai-oauth/releases/tag/v0.1.1`.

## Verification

Run:

```powershell
python .\scripts\verify-grok-build-composer-capabilities.py
```

The catalog-repair project must additionally run its evidence validator,
profile test, read-only runtime audit, and a second dry-run after projection.
Context-window fields are outside the scope of these rules and must remain
operator-owned.
