# Thapthim Architecture

How Thapthim turns spaceless Thai text into word and syllable boundaries. Short version: the **How
it works** section in the [README](../README.md); numbers: [BENCHMARKS.md](BENCHMARKS.md).

## Summary

Thapthim segments in a single near-linear sweep anchored to one substrate: the **Thai Character
Cluster (TCC) grid**. A rule-based regex segmenter tiles the raw bytes into TCCs — the smallest
orthographically inseparable units — holding markup and western runs (URLs, decimals, Thai-numeral
sequences, `@`-handles, `#`-tags, identifiers) together as single clusters. This grid is both the
internal coordinate system and a public capability (`tcc_segment`).

Over the grid, a character-wise Double-Array Aho-Corasick automaton (`daachorse::charwise`), built
from a unified dictionary (LST20 ∪ BEST ∪ PyThaiNLP), emits every overlapping **word** and
**syllable** candidate up to `THAPTHIM_MAX_WORD_TCC` clusters (default 12, set where LST20 accuracy
plateaus across 10–12). Candidates snap to the grid and fill a flat `Vec`-backed multi-granularity
lattice. The decode relies on the invariant that every word boundary is a TCC boundary
(`word ⊂ TCC`) — exact because the grid is computed by deterministic rule (see below).

Transitions are scored by a **Kneser-Ney bigram LM trained solely on LST20** (one consistently
annotated backbone). It is interned into dense integer ids with bigram keys packed into 64-bit words
(`w1 << 32 | w2`), with follower/continuation counts (N₁₊) precomputed at bootstrap so the hot path
never rescans n-grams. (Keys use a splitmix64 finalizer to avoid low-bit clustering — a ~3× decode
win.)

Decoding is **word-first**: one Viterbi pass picks the optimal word path. Spans the dictionary can't
cover are priced through an **OOV back-off** with a tunable penalty (`THAPTHIM_OOV_PENALTY`, default
2.0), so an unattested dictionary entry can't out-score the decomposition it should defer to.
Over-segmented unknown runs are then repaired by a **branching-entropy pass**
(`THAPTHIM_BE_THRESHOLD`): a forward/backward character-successor model (Shannon entropy from the
TNHC corpus) dissolves spurious internal cuts in coined/foreign words while leaving genuine
high-entropy boundaries intact. A single bare Thai consonant is treated as OOV when coalescing, so it
can't tile an unknown run into sub-syllabic fragments (`บลัช` → `บ`·`ลัช`, not `บ`·`ลั`·`ช`);
genuine one-letter words (`ณ`, `ธ`) re-emerge unchanged, so the rule is boundary-neutral.

Word and syllable resolution are **two independent passes** for speed — the word pass lazily
syllabifies only the OOV spans it must, while a dedicated syllable Viterbi yields a full
orthographic-syllable tiling, both on the shared grid. Output is a flat vector of packed 64-bit
tokens, `[ Start | Length | Tier ]`, referencing the original byte buffer.

## The grid invariant: `word ⊂ TCC`, not `word ⊂ syllable`

All three granularities nest in the *gold* segmentation (word ⊂ syllable ⊂ TCC) — a fact about Thai,
not something the engine computes. The decoder leans only on the **outer** half, `word ⊂ TCC`, and
anchors on TCC rather than syllables because only one level is reproducible *exactly*:

- **TCC computed = TCC gold**, exactly (deterministic regex, no probabilities). So `word ⊂ TCC`
  holds with certainty; the grid never excludes a boundary the word decode wants.
- **Syllables computed ≠ syllables gold** — syllabification is a probabilistic guess. Constraining
  word candidates to a *guessed* syllabification forecloses 0.15–1.6% of gold word boundaries
  (measured across LST20/BEST/VISTEC/TNHC/ws1000) before the word LM runs — a recall ceiling the TCC
  grid does not impose.

So the engine is a word-first **cascade** over a deterministic grid, not a joint word⊗syllable
decode: it never commits to an ambiguous syllable cut upstream of the word decision. (The lazy
syllable pass only subdivides *within* spans whose edges are already fixed.)

## Pipeline

```
========= BUILD TIME =================================================

  Kneser-Ney count tables   ─build.rs─>   interned bigram LM
  (word / syll / TCC tiers,               (dense u32 ids, packed-u64
   LST20 train ONLY)                       bigrams, N1+ precomputed)  ─┐
                                                                       │
  char-successor entropy table  (fwd/bwd Shannon, from TNHC)  ────────>├─ embedded
                                                                       │  in binary
  master dictionary  (word + syllable vocab: LST20 ∪ BEST ∪ PyThaiNLP) ┘

                              │  deserialized once at bootstrap
                              v
========= RUNTIME ==================================================

                   +-----------------------------+
                   |        RAW THAI TEXT         |
                   +-----------------------------+
                                 |
                                 v
                   +-----------------------------+
                   | TCC SEGMENTER (rule-based   |
                   | regex): atomic clusters;    |
                   | <tags>, URLs, decimals,     |
                   | Thai-numeral runs, @handles,|
                   | #tags held whole            |
                   +-----------------------------+
                                 | TCC grid (atomic byte boundaries)
                                 | ── Ruby: tcc_segment / tcc_positions
                                 v
                   +-----------------------------+
                   | daachorse charwise trie     |
                   | (dict): overlapping word +  |
                   | syll candidates, grid-      |
                   | snapped, len <= 12 TCC  *(1)|
                   +-----------------------------+
                                 |
                                 v
                   +-----------------------------+
                   | MULTI-GRANULARITY LATTICE   |
                   | (flat Vec): candidates snap |
                   | to TCC grid  (word ⊂ tcc)   |
                   +-----------------------------+
                          |                  |
                          v                  v
              +---------------------+  +---------------------+
              | WORD PASS           |  | SYLLABLE PASS       |  <-- independent
              | Viterbi, word-KN LM |  | Viterbi + KN LM,    |      pass (speed):
              | + OOV back-off      |  | full-text tiling    |      no word work
              | (penalty 2.0) *(2)  |  +---------------------+      needed
              +---------------------+             |
                          |                       |
                          v                       |
              +---------------------+             |
              | coalesce OOV runs   |             |
              | -> lazy syllabify   |             |
              +---------------------+             |
                          |                       |
  char_entropy            v                       |
  (fwd/bwd) ──> +---------------------+           |
                | branching-entropy   |           |
                | merge: len <= 2 TCC |           |
                | *(3), H < 1.0 *(4)  |           |
                +---------------------+           |
                          |                       |
                          v                       v
              +---------------------+  +---------------------+
              | packed u64 WORD toks|  | packed u64 SYLL toks|
              | [ Start|Length|Tier]|  | [ Start|Length|Tier]|
              +---------------------+  +---------------------+
                          |                       |
                          v                       v
                  Thapthim.word_segment        Thapthim.syllable_segment
                          |                       |
                          +── byteslice original buffer ──+

 * runtime knobs (env var, no rebuild):
     (1) THAPTHIM_MAX_WORD_TCC = 12   (TCC clusters, not chars)
     (2) THAPTHIM_OOV_PENALTY  = 2
     (3) THAPTHIM_BE_MAX_TCC   = 2    (max token length eligible to merge)
     (4) THAPTHIM_BE_THRESHOLD = 1.0  (0 disables the merge pass)
```

## Assets

Literal files under `ext/thapthim/assets/`, all committed in-repo and built offline by the corpus
notebook — except the interned `.bin`s, which `build.rs` regenerates from the count tables at build.

| concept | file(s) | role |
|---|---|---|
| KN count tables | `kn_{words,syllables,tccs}_{unigrams,bigrams}.txt` | human-readable KN counts; `build.rs` input (LST20 train only) |
| interned bigram LM | `joint_lm_interned.bin` | compact embedded LM (default, LST20); loaded at runtime (`joint_lm.bin` is the build.rs intermediate) |
| interned LM _(gated)_ | `joint_lm_interned_{best,combined}.bin` | alternate LMs, embedded only under the `best_lm`/`combined_lm` cargo feature |
| entropy table | `char_entropy.txt` | fwd/bwd Shannon entropy for the branching-entropy merge (from TNHC) |
| master dictionary | `master_{words,syllables}_vocab.txt` | word + syllable vocab (LST20 ∪ BEST ∪ PyThaiNLP) |

## Runtime knobs

| env var | default | effect |
|---|--:|---|
| `THAPTHIM_MAX_WORD_TCC` | 12 | max dictionary candidate length, in TCC clusters (`0` = no cap) |
| `THAPTHIM_OOV_PENALTY` | 2.0 | log-prob penalty on transitions out of an LM-unseen word |
| `THAPTHIM_BE_MAX_TCC` | 2 | max token length (TCC clusters) eligible for a branching-entropy merge |
| `THAPTHIM_BE_THRESHOLD` | 1.0 | branching-entropy merge threshold (`0` disables the pass) |
| `THAPTHIM_KN_DISCOUNT` | 0.75 | Kneser-Ney absolute discount (argmax near-invariant; see BENCHMARKS) |
| `THAPTHIM_LM` | LST20 | selects the LM tier when a gated `best`/`combined` build is loaded |
| `THAPTHIM_WORD_VOCAB` | _(embedded)_ | swap the word dictionary for an external one-word-per-line file (eval/custom-dict) |

## Extensibility

The Viterbi core in `lattice/grid.rs` is task-agnostic: an `Edge<P>` is a grid-aligned span carrying
an arbitrary payload `P`, and the `LatticeModel` trait supplies the cost (`start_ctx`, optional
`node_cost`, first-order `transition`). Per-node context is resolved up front by the candidate
builder, so the trait stays cost-only and is monomorphized per model (no dynamic dispatch).

Word/syllable segmentation is the first instantiation: `build_lattice` (in `decode.rs`) emits
candidates, and `BigramModel` sets `transition =` the KN bigram score. The branching-entropy merge
and OOV coalescing stay *outside* the core as segmentation-specific orchestration. Planned
deterministic tasks plug in the same way (new candidates + a `LatticeModel`, never touching
`grid.rs`):

- **G2P** — edges are span→reading candidates; `transition` a phoneme-sequence model.
- **Spelling correction** — edges are dictionary words within an edit-distance bound; `node_cost`
  carries the edit penalty (first real use of `node_cost`); `transition` the word LM.

Tasks with no path search (soundex, ISO-11940 transliteration, normalization) are deterministic
transforms beside the lattice, not inside it.
