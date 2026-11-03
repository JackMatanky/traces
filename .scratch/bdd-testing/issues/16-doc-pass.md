# 16: Doc pass: ADR and glossary corrections

**What to build:** The ADRs and glossaries agree with the code. Read spec sections 6.7 and 9.8. Coordinate with arch-design ticket 23.

**Current state:** Two ADR files share one number. The glossaries do not mark slugs, callouts and footnotes as unimplemented. The template glossary has the wrong loader order.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 06 Coverage-gap bookkeeping and slice 1 close

**Category:** enhancement

**Status:** ready-for-agent

- [ ] One of the two ADR-0009 files is renumbered to 0010 and every reference follows. Neither is marked superseded.
- [ ] In-tree markers state that slugs, callouts and footnotes have no code.
- [ ] The template glossary states the real loader order.
- [ ] The ADR corrections of spec section 6.7 (items 1, 2 and 6) are applied.
- [ ] `mise run verify` passes.

**Out of scope:**
- Marking either ADR superseded.
- Implementing slugs, callouts or footnotes.
