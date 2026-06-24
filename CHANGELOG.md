## [Unreleased]

- Generalized the segmentation lattice into a task-agnostic grid engine (`lattice/grid.rs`:
  `Edge<P>`, the `LatticeModel` trait, and `viterbi`), with word/syllable segmentation as its
  first instantiation (`BigramModel`). Output is bit-identical (LST20 F1 0.9476, TNHC 0.7916,
  0 reconstruction mismatches). Lays the groundwork for G2P and spelling correction as further
  `LatticeModel` instantiations.
- Word/syllable segmentation throughput up ~55% (word ~1.68M → ~2.6M char/s, syllable
  ~2.24M → ~3.5M char/s) from a series of hot-path cleanups: allocation-free grid candidates
  (no per-candidate owned `String`), a flat `byte → TCC-index` array replacing the per-match
  hashmap probes in candidate generation (also the grid-membership test, so no separate boundary
  set), an unstable sort of the Viterbi node list instead of a stable clone, per-thread reuse of
  the Viterbi DP scratch buffers, and the pre-existing splitmix64 bigram-key hasher + id precompute.
  All accuracy numbers unchanged (LST20 F1 0.9476, TNHC 0.7916, 0 reconstruction mismatches).

## [0.1.0] - 2026-06-09

- Initial release
