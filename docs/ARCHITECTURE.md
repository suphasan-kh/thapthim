# Thapthim Architecture

How Thapthim turns spaceless Thai text into word and syllable boundaries — the full pipeline,
end to end. For the short version see the **How it works** section in the [README](../README.md);
for accuracy and speed numbers see [BENCHMARKS.md](BENCHMARKS.md).

## Summary

Thapthim segments Thai text in a single near-linear sweep that stays anchored to one substrate:
the **Thai Character Cluster (TCC) grid**. A rule-based regular-expression segmenter
first tiles the raw bytes into TCCs — the smallest orthographically inseparable units — holding
angle-bracket markup and western runs (URLs, decimals, Thai-numeral sequences, `@`-handles,
`#`-tags, identifiers) together as single clusters, matching the syllabifier's own western-token
convention. This grid is both the internal coordinate system and a first-class capability exposed
to Ruby (`tcc_segment`).

Over that grid, a character-wise Double-Array Aho-Corasick automaton (`daachorse::charwise`), built
from a unified dictionary (LST20 ∪ BEST ∪ the PyThaiNLP lexicon), streams the text and emits every
overlapping **word** and **syllable** candidate up to a bounded length — `THAPTHIM_MAX_WORD_TCC`
(default 12, counted in TCC clusters, fixed empirically where LST20 accuracy plateaus across 10–12,
not read off a paper's character percentile). Each candidate snaps to the grid; the
load-bearing invariant the decode uses is that every word boundary is a TCC boundary
(`word ⊂ TCC`), which holds *exactly* because the grid is computed by deterministic rule, never a
probabilistic guess. (The fuller nesting — every word boundary also a syllable boundary, every
syllable boundary a TCC boundary — is a property of the *gold* segmentation, not something the word
pass computes; see **The grid invariant** below.) The candidates fill a flat, `Vec`-backed
multi-granularity lattice.

Transitions are scored by a **Kneser-Ney–smoothed bigram language model trained solely on LST20** —
one consistently annotated backbone, free of cross-criteria contamination. The model is interned
into dense integer token-ids with bigram keys folded into packed 64-bit words
(`w1_id << 32 | w2_id`), collapsing hundreds of thousands of heap-allocated string keys into a
compact table; per-context follower and continuation counts (N₁₊) are compiled once at bootstrap so
the hot path never rescans the n-grams, and the interned asset is regenerated in-repo at build time
from human-readable KN count tables. (The u64 keys are hashed with a splitmix64 finalizer to defeat
the low-bit clustering that a plain multiply-hash exhibits on structured keys — a ~3× decode win.)

Decoding is **word-first**: one Viterbi pass with full back-pointer recovery selects the globally
optimal word path. Spans the dictionary cannot cover are priced through an explicit
**out-of-vocabulary back-off** carrying a tunable penalty (`THAPTHIM_OOV_PENALTY`,
default 2.0), so an unattested dictionary entry can no longer out-score the decomposition it ought to
defer to. Over-segmented unknown runs are then repaired by a **branching-entropy pass**
(`THAPTHIM_BE_THRESHOLD`): a forward/backward character-successor model (Shannon entropy harvested
offline from the TNHC literary corpus) dissolves spurious internal cuts in coined or foreign words,
fusing short low-entropy fragments while leaving genuine high-entropy boundaries intact.

A single bare Thai consonant (the vocab carries all 43 as degenerate one-letter entries) is treated
as OOV when coalescing, not as a real word, so it cannot tile an unknown run into sub-syllabic
fragments (`บลัช` → `บ`·`ลัช`, never `บ`·`ลั`·`ช`); a genuine isolated one-letter word (`ณ`, `ธ`)
just forms a length-1 OOV span and re-emerges as the same token, so the rule is boundary-neutral.

Word and syllable resolution are **decoupled into two independent passes** for speed — the word pass
lazily syllabifies only the OOV spans it must, while a dedicated syllable Viterbi over the same
backbone yields a full orthographic-syllable tiling — both anchored to the shared TCC grid. The
reconciled path is emitted via native bit-shifting as a flat vector of packed 64-bit tokens,
`[ Start | Length | Tier ]`, referencing the original byte buffer.

## The grid invariant: `word ⊂ TCC`, not `word ⊂ syllable`

The three granularities nest in the *gold* segmentation — every true word boundary is a true
syllable boundary, every true syllable boundary a true TCC boundary — because a word is built from
whole syllables and a syllable from whole TCCs. That nesting is a fact about Thai: true of any
correct analysis, independent of any algorithm. It is **not** something the engine computes or
maintains.

The decoder leans on only the **outer** half of it, `word ⊂ TCC`. Word candidates snap to the TCC
grid, and that costs nothing precisely because every gold word boundary is a gold TCC boundary — the
grid can never exclude a boundary the word decode would want. The word pass goes **straight from TCC
to word**; it never computes syllables. (Syllables appear only in the lazy second pass over spans
the word decode already left OOV, where they subdivide *within* a span whose edges are already
fixed — so even there nothing relies on `word ⊂ syllable` to recover a word boundary.)

Why anchor on TCC rather than syllables, when the gold nesting holds for both? Because only one
level is *reproducible exactly* by the engine:

- **TCC the engine computes = TCC gold**, exactly — the deterministic regex grammar, no
  probabilities. So `word ⊂ TCC_computed` holds with certainty; the grid is the true superset, and
  aligning word candidates to it is lossless.
- **Syllables the engine would compute ≠ syllables gold** — syllabification is a probabilistic
  decision (a trained Viterbi/CRF guess). So `word ⊂ syllable_computed` can fail even though
  `word ⊂ syllable_gold` is exact. Constraining word candidates to a *guessed* syllabification
  forecloses 0.15–1.6% of gold word boundaries (measured across LST20, BEST, VISTEC, TNHC, ws1000)
  before the word LM ever runs — a recall ceiling the deterministic TCC grid does not impose.

Stated as architecture, this is why the engine is a word-first **cascade** over a deterministic grid
rather than a joint word⊗syllable decode: it never commits to an *ambiguous* syllable cut upstream
of the word decision.

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

The diagram above names assets conceptually; these are the literal files under
`ext/thapthim/assets/`. All are committed in-repo and produced offline by the corpus
notebook, except the two interned `.bin`s, which `build.rs` regenerates from the count
tables at build time.

| concept in diagram | file(s) | role |
|---|---|---|
| Kneser-Ney count tables | `kn_{words,syllables,tccs}_{unigrams,bigrams}.txt` | human-readable KN counts; `build.rs` input (LST20 train only) |
| _(intermediate)_ | `joint_lm.bin` | string-keyed LM emitted by `build.rs` step 1 |
| interned bigram LM | `joint_lm_interned.bin` | compact embedded LM (default, LST20); the one loaded at runtime |
| interned bigram LM _(gated)_ | `joint_lm_interned_{best,combined}.bin` | alternate LMs, embedded only under the `best_lm`/`combined_lm` cargo feature |
| char-successor entropy table | `char_entropy.txt` | fwd/bwd Shannon entropy for the branching-entropy merge (from TNHC) |
| master dictionary | `master_{words,syllables}_vocab.txt` | word + syllable vocab (LST20 ∪ BEST ∪ PyThaiNLP) |

## Runtime knobs

| env var | default | effect |
|---|--:|---|
| `THAPTHIM_MAX_WORD_TCC` | 12 | max dictionary candidate length, counted in TCC clusters (`0` = no cap) |
| `THAPTHIM_OOV_PENALTY` | 2.0 | log-prob penalty on transitions out of an LM-unseen word (OOV back-off) |
| `THAPTHIM_BE_MAX_TCC` | 2 | max token length (TCC clusters) eligible for a branching-entropy merge |
| `THAPTHIM_BE_THRESHOLD` | 1.0 | branching-entropy merge threshold for OOV-run repair (`0` disables the pass) |
| `THAPTHIM_KN_DISCOUNT` | 0.75 | Kneser-Ney absolute discount (argmax is near-invariant; see BENCHMARKS) |
| `THAPTHIM_LM` | LST20 | selects the LM tier when a gated `best`/`combined` build is loaded |

## Extensibility

The Viterbi decoder is not specific to segmentation. The shortest-path core lives in
`lattice/grid.rs` as a task-agnostic engine: an `Edge<P>` is a grid-aligned span `[start, end)`
carrying an arbitrary payload `P`, and the `LatticeModel` trait supplies the cost — a sentence-initial
context (`start_ctx`), an optional emission/node cost (`node_cost`, default `0.0`), and a first-order
transition score (`transition`). Each edge's per-node context (`Ctx`) is resolved up front by the
candidate builder and handed to `viterbi` as the `ctx` slice, so the trait stays cost-only. `viterbi`
runs the exact first-order DP over the edges confined to a byte region and returns the best path. The
trait is monomorphized per model, so this generality costs no dynamic dispatch.

Word and syllable segmentation are simply the **first** instantiation: `build_lattice` in `decode.rs`
emits the candidates with their contexts, and `BigramModel` sets `Payload = LatticeTier`,
`Ctx = Option<u32>` (the interned token id), `node_cost = 0.0`, and `transition = ` the Kneser-Ney
bigram score. The branching-entropy merge and OOV-run coalescing stay *outside* the core as
segmentation-specific orchestration.

The planned deterministic tasks plug in the same way — new candidate generation plus a `LatticeModel`
impl, never touching `grid.rs`:

- **G2P** — edges are span→reading candidates; `Ctx` is a phoneme-unit id; `node_cost` carries a
  grapheme→phoneme rule prior; `transition` is a phoneme-sequence model.
- **Spelling correction** — edges are dictionary words within an edit-distance bound; `node_cost`
  carries the edit penalty (the first real use of `node_cost`); `transition` is the word LM.

Tasks with no path search — soundex, ISO-11940 transliteration, normalization, sentiment — are
deterministic transforms that sit beside the lattice rather than inside it.
