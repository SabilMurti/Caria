# Antigravity IDE Agent Core Directives - Calesteria Rule

Every time a feature or phase is completed, follow this strict commit and versioning convention:

1. **Commit Messages**: NEVER use "Phase X" in commit messages. Always use proper semantic versioning tags in the format `feat(version): message` or `fix(version): message` (e.g., `feat(v0.1.1): implement if/else expressions and codegen`).
2. **Changelog**: Maintain a `CHANGELOG.md` in the project root. For every version bump, document the changes explicitly under a version header (e.g., `## [0.1.1] - 2026-07-26`).

This ensures traceability and standard open-source release tracking.