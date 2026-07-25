#!/usr/bin/env python3
"""Verify the tracked Grok Build/Composer capability overrides."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PROFILE_PATH = ROOT / "runtime" / "codex-model-capability-profiles.json"

EXPECTED = {
    "grok-build": ("reasoning_effort", ["low", "medium", "high"], "high"),
    "grok-build-latest": (
        "reasoning_effort",
        ["low", "medium", "high"],
        "high",
    ),
    "grok-build-0.1": ("none", ["high"], "high"),
    "composer-2.5": ("none", ["none"], "none"),
    "grok-composer": ("none", ["none"], "none"),
    "grok-composer-2.5-fast": ("none", ["none"], "none"),
}


def resolve(profile: dict, slug: str) -> dict:
    capability: dict = {}
    for rule in profile.get("model_rules", []):
        if any(re.search(pattern, slug, flags=re.IGNORECASE) for pattern in rule.get("patterns", [])):
            capability.update(rule)
    capability.update(profile.get("model_overrides", {}).get(slug, {}))
    return capability


def main() -> int:
    profile = json.loads(PROFILE_PATH.read_text(encoding="utf-8-sig"))
    failures: list[str] = []

    for slug, expected in EXPECTED.items():
        capability = resolve(profile, slug)
        actual = (
            capability.get("reasoning_transport"),
            capability.get("reasoning_levels"),
            capability.get("default_reasoning_level"),
        )
        if actual != expected:
            failures.append(f"{slug}: {actual!r} != {expected!r}")

        evidence = capability.get("evidence") or []
        if not evidence or any(not str(url).startswith("https://") for url in evidence):
            failures.append(f"{slug}: missing HTTPS evidence")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1

    for slug, (transport, levels, default) in EXPECTED.items():
        print(f"PASS: {slug}: transport={transport}, levels={levels}, default={default}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
