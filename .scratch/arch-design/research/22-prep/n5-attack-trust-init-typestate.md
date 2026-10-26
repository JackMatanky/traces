## Attack Notes — Trust Init and Typestate Invariants

Focus: Q4–Q7, covering trust config invariant, init, typestate invariants, crash safety, and cross-ticket conflicts.

---

### 1. Stress-Test: Trust Invariant and Crash Analysis (Q4, Q5)

#### Decision Summary

- **Invariant**: Tracker entry exists before a trusted root record is established. Both must co-exist.
- Abolished root-only grants.
- Commands (`traces untrust`, `traces trust --all`) must respect these guarantees.
- Key API paths:
  - `ConfigService::trust`.
  - Companion writes.

#### Attack Factors

**Crash Points**:

- Examine concurrent racing for `trust` vs `tracked clean` in `src/config/service.rs:traced_subjects`.
- Following `FilePathTracker::record` (possible symlink `EEXIST` case).

**Missing Tests:**

- Test with `.traces` symlinked OR stale-file triggers via key mismatch `/stale`.
- Test racing subprocess “lost” `.traces/` artifacts rewrite due to CI init/reinit.
- **Trust w/o Baselines** Case scenarios (e.g., Windows file remap artifacts disappear).

---

### TBI 6 Typ
