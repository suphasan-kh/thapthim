## [Unreleased]

- **Fix: single bare consonants no longer fragment OOV runs into sub-syllabic junk.** The word
  vocabulary carries all 43 Thai consonants as degenerate one-letter entries, which let the word
  Viterbi tile an unknown transliteration with them — e.g. `บลัชออน` → `บ`·`ลั`·`ช`·`ออน`, leaking a
  bare TCC cluster (`ลั`, an incomplete syllable) into the word output. The OOV-span coalescer now
  treats a one-letter-consonant "word" as out-of-vocabulary and folds it into the adjacent OOV run,
  so the whole run is syllabified as a unit (`บ`·`ลัช`·`ออน`). Genuine one-letter words (`ณ`, `ธ`)
  are unaffected: an isolated one becomes a length-1 OOV span and re-emerges as the identical surface
  token (boundary-neutral). Word-span F1 (incl. spaces) improves on every corpus with no regression —
  TNHC 0.7908→0.7919, LST20 0.9552→0.9555, BEST 0.8738→0.8739, VISTEC 0.8257→0.8283 (heaviest OOV
  tail, largest gain); precision up across the board. In-vocabulary segmentation is byte-identical.
- **Breaking (pre-1.0): renamed the segmentation API for consistency** — every entry point is now
  `<unit>_segment`. Ruby: `segment` → `word_segment`, `syllables` → `syllable_segment` (`tcc_segment`
  unchanged). Python: `segment` → `word_segment`, `syllables` → `syllable_segment`, `segment_offsets`
  → `word_segment_offsets`, `segment_batch` → `word_segment_batch`. The `normalize:`/`normalize=`
  options and all behavior are unchanged.
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
