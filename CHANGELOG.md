## [Unreleased]

- **Fix: Thai abbreviation periods no longer get stolen by an adjacent number.** The western-token
  TCC rule was an unanchored class (`[A-Za-z0-9_.,@:/#-]+`), so a connector like `.` could *lead* a
  cluster — e.g. `พ.ศ.2568` split as `พ.ศ`·`.2568`, emitting a token that starts with a stray period
  and leaving `พ.ศ.` unrecoverable by the dictionary. The connectors (`. , @ : / # -`) are now
  ANCHORED between two alphanumeric runs (`[@#]?[A-Za-z0-9_]+(?:[.,@:/#-]+[A-Za-z0-9_]+)*`), with
  `@`/`#` the only ones that may lead (handles, tags). Dates/abbreviations now segment correctly
  (`พ.ศ.`·`2568`, `กทม.`·`50000`), and all western tokens (`3.5`, `5-6`, phone numbers, URLs,
  `@handle`, `#tag`, `covid19`) are unchanged. Word-span F1 (incl. spaces) up on every corpus —
  TNHC 0.7919→0.7945, LST20 0.9555→0.9571, BEST 0.8739→0.8754, VISTEC 0.8283→0.8291; precision and
  recall both up, 0 reconstruction mismatches.
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
- Word/syllable segmentation throughput up ~75% (word ~1.68M → ~2.9M char/s, syllable
  ~2.24M → ~4.0M char/s; Python single-core ~3.2M / ~4.9M, batch ~11M on 8 cores) from a series of
  output-identical hot-path improvements: allocation-free grid candidates (no per-candidate owned
  `String`); a flat `byte → TCC-index` array replacing the per-match hashmap probes in candidate
  generation (also the grid-membership test, so no separate boundary set); dictionary-candidate
  contexts resolved by id from a precomputed line-index→token-id table instead of re-hashing
  surfaces; dense grid-index Viterbi predecessor buckets with a counting sort over the distinct
  start positions (replacing a per-node byte-keyed hashmap probe and an O(n log n) node sort); a
  single `build_lattice` shared by the word, syllable, and OOV-span decodes; per-thread reuse of the
  Viterbi DP scratch buffers; and the splitmix64 bigram-key hasher. Accuracy unchanged (LST20 F1
  0.9476, TNHC 0.7916, 0 reconstruction mismatches), verified byte-identical by a digest over all
  corpora.

## [0.1.0] - 2026-06-09

- Initial release
