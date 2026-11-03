# 15: U1: four yaml.rs unit pins

**What to build:** Four unspecified yaml.rs behaviors get unit tests. Read spec section 9.8.

**Blocked by:** 06 Coverage-gap bookkeeping and slice 1 close

**Category:** enhancement

**Status:** ready-for-agent

- [ ] For each of the four claims the code is read first. When the code and the documentation disagree, the code wins and the documentation is corrected.
- [ ] A unit test pins that binary literals read as integers.
- [ ] A unit test pins that an overflowing float stays a string.
- [ ] A unit test pins that `u64::MAX` is an error.
- [ ] A unit test pins that a non-scalar key is an error.
- [ ] `mise run verify` passes.
