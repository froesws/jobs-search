# Contracts

- [`tauri-commands.md`](./tauri-commands.md) — backend command surface consumed
  by `002`.
- [`mcp-jobspy.md`](./mcp-jobspy.md) — stdio JSON-RPC contract for the Python
  JobSpy sidecar.
- [`llm-profile-extraction.schema.json`](./llm-profile-extraction.schema.json) —
  strict profile extraction response.
- [`llm-evaluation.schema.json`](./llm-evaluation.schema.json) — requirement and
  per-dimension evidence; the app computes the final score.

All contracts are local. No contract includes credentials, raw job descriptions,
model prompts, or candidate data beyond the profile/evidence needed for the
operation.
