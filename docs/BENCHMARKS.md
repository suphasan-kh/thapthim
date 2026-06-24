# Thapthim Benchmarks

Accuracy and speed of Thapthim against the common Thai word-segmentation engines, scored with
the **research-standard metric** used by the AttaCut and DeepCut papers — the
`pythainlp.benchmarks` reference implementation. Two F1 levels are reported (whitespace is
stripped before scoring, per the reference preprocessing):

- **char-level F1** — word-boundary detection: every character is labelled start-of-word or not.
- **word-level F1** — a predicted word is correct iff **both** of its boundaries match the gold word.

Last run: 2026-06-24 · Apple M1 (8 cores) · pythainlp 5.3.4, attacut 1.0.6 (`attacut-sc`),
deepcut 0.7.0.0 (TensorFlow 2.21), nlpo3 1.4.0.

## Engines

| engine | type | training standard |
|---|---|---|
| **thapthim-LST20** | dictionary lattice + KN-bigram Viterbi (this gem, shipped default) | LST20 |
| **thapthim-BEST** | same engine, alternate LM (gated `best_lm` feature, not shipped) | BEST |
| **thapthim-COMBINED** | same engine, LST20∪BEST word LM (gated `combined_lm` feature, not shipped) | LST20+BEST |
| attacut (`attacut-sc`) | neural (CNN) | BEST |
| deepcut | neural (CNN/LSTM) | BEST |
| nlpo3 | dictionary maximal-matching (Rust newmm) | LEXiTRON-style dict |
| newmm | dictionary maximal-matching (PyThaiNLP) | LEXiTRON-style dict |

Full test sets — lst20 5,250 · best 27,834 · vistec 10,000 · tnhc 4,403 · ws1000 993 sentences.
Columns are the three thapthim LMs (LST20 ships; BEST and COMBINED are gated, not shipped — same
engine, different training corpus) and the four baselines. All engines reconstructed every sentence
exactly (0 mismatches).

## Word-level F1 (research metric; **bold** = best per corpus)

| corpus | LST20 | BEST | COMBINED | attacut | deepcut | nlpo3 | newmm |
|---|--:|--:|--:|--:|--:|--:|--:|
| **lst20**  | **0.9481** | 0.8698 | 0.9361 | 0.8532 | 0.8522 | 0.7135 | 0.7124 |
| **best**   | 0.8755 | 0.9496 | 0.9246 | 0.9454 | **0.9659** | 0.6870 | 0.6839 |
| **vistec** | **0.8124** | 0.8050 | 0.8072 | 0.7843 | 0.7971 | 0.7480 | 0.7667 |
| **tnhc**   | 0.7916 | **0.8077** | 0.8076 | 0.7667 | 0.7764 | 0.7084 | 0.7095 |
| **ws1000** | 0.8280 | 0.8288 | **0.8330** | 0.8261 | 0.8243 | 0.7525 | 0.7487 |
| **macro-avg** | 0.8511 | 0.8522 | **0.8617** | 0.8351 | 0.8432 | 0.7219 | 0.7242 |

## Char-level F1 (boundary detection)

| corpus | LST20 | BEST | COMBINED | attacut | deepcut | nlpo3 | newmm |
|---|--:|--:|--:|--:|--:|--:|--:|
| **lst20**  | **0.9781** | 0.9496 | 0.9738 | 0.9420 | 0.9413 | 0.8901 | 0.8899 |
| **best**   | 0.9500 | 0.9763 | 0.9674 | 0.9771 | **0.9865** | 0.8770 | 0.8747 |
| **vistec** | **0.9212** | 0.9182 | 0.9192 | 0.9146 | 0.9192 | 0.8970 | 0.9060 |
| **tnhc**   | 0.9178 | **0.9237** | 0.9237 | 0.9006 | 0.9068 | 0.8873 | 0.8876 |
| **ws1000** | 0.9292 | 0.9289 | 0.9304 | **0.9316** | 0.9307 | 0.9006 | 0.9024 |

Among thapthim's LMs (engine identical, only the training corpus differs): each single-corpus LM wins
its home corpus, while **COMBINED** is the best all-rounder — highest word-F1 macro-average (0.862)
of *any* engine here, and it never collapses, trading a little LST20/VISTEC peak for large BEST/TNHC
gains. The shipped default is LST20 (highest home peak). The KN absolute discount was swept over
0.1–0.99 on all three LMs with no meaningful effect (argmax decoding is near-invariant to a uniform
score shift), so the textbook `d = 0.75` is retained.

## OOV recall — generalization to unknown words

Word-level F1 above is dominated by frequent, in-dictionary words and hides how each engine
handles words it has never seen. This section stratifies **recall** by dictionary membership
(SIGHAN-style). One **shared OOV reference** is used for every engine — Thapthim's shipped word
lexicon (`ext/thapthim/assets/master_words_vocab.txt`, 141,548 words): a gold word is **OOV** iff
it is absent from that set. Every engine is therefore scored on the *identical* OOV word set, so
the comparison is apples-to-apples ("of the words Thapthim doesn't know, how many does each model
recover?"). A gold word is recalled iff its exact `[start,end)` span appears in the prediction;
whitespace tokens are excluded. Caps match the dump (lst20 5,250 · best 3,000 · vistec 3,000 ·
tnhc 4,403 · ws1000 993) to bound deepcut's runtime. Last run: 2026-06-25.

### R_oov — recall on out-of-vocabulary words (**bold** = best per corpus)

| corpus | OOV% | LST20 | BEST | COMBINED | attacut | deepcut | nlpo3 | newmm |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| **lst20**  | 1.6%  | 0.1558 | 0.1558 | 0.1558 | 0.3282 | **0.3506** | 0.1494 | 0.1486 |
| **best**   | 0.8%  | 0.0667 | 0.0667 | 0.0667 | 0.3833 | **0.5262** | 0.0905 | 0.0452 |
| **vistec** | 10.0% | 0.2579 | 0.2579 | 0.2578 | 0.2661 | **0.4228** | 0.1858 | 0.2782 |
| **tnhc**   | 5.0%  | 0.2516 | 0.2512 | 0.2512 | 0.3121 | **0.3735** | 0.2729 | 0.2810 |
| **ws1000** | 10.4% | 0.4216 | 0.4206 | 0.4206 | 0.5179 | **0.5415** | 0.4191 | 0.3863 |
| **micro-avg** | 5.1% | 0.2527 | 0.2525 | 0.2524 | 0.2992 | **0.4157** | 0.2128 | 0.2682 |

### R_iv — recall on in-vocabulary words (micro-avg)

| | LST20 | BEST | COMBINED | attacut | deepcut | nlpo3 | newmm |
|---|--:|--:|--:|--:|--:|--:|--:|
| **R_iv** | 0.8960 | 0.8958 | **0.9086** | 0.8554 | 0.8601 | 0.7280 | 0.7262 |

**OOV recall is Thapthim's weakest dimension.** At ~0.25 micro-avg it sits in the *dictionary
tier* — slightly ahead of nlpo3 (the branching-entropy merge earns a real edge), roughly level
with newmm, and behind both neural models: ~18% below attacut and **~64% below deepcut**, which
recovers 1.6× as many unknown words. This is the dictionary-model trade-off: the same approach
that gives Thapthim the **best in-vocab recall of any engine here** (0.896, and 0.909 with the
combined LM) cannot invent boundaries for words absent from its lexicon the way a sub-word neural
model can. OOV is hard for everyone on this data — even deepcut only clears 0.42.

**The LM corpus barely touches OOV recall** — all three LMs land within 0.0003 micro
(LST20 0.2527 · BEST 0.2525 · COMBINED 0.2524; per-corpus deltas ≤ 0.001). OOV merging is driven
by the LM-independent branching-entropy post-pass, not the word bigram LM, so swapping LMs leaves
it flat. The LM instead moves **in-vocab** disambiguation: each single-corpus LM peaks R_iv on its
home corpus (LST20→lst20, BEST→best 0.955), and the combined LM is the best all-rounder (micro
R_iv 0.896 → 0.909; BEST 0.865 → 0.925), which is where its word-F1 gains come from. This also explains Thapthim's strong **cross-domain** F1: candidate words come from a
broad-domain *union* dictionary (LST20∪BEST∪PyThaiNLP) and OOV handling is domain-general, so the
parts that carry accuracy don't actually depend on the LM's home corpus — consistent with the
flat LM sweep noted above. The headline F1 lead survives the weak OOV recall because OOV rates are
low (0.8–10%) and in-vocab recall is dominant; OOV only bites on the high-OOV corpora (vistec,
ws1000), which is exactly where deepcut closes the F1 gap.

Reproduce: `ruby test/eval_oov.rb` (Thapthim alone, shipped LM) or, for the cross-model table,
generate the dumps as in [Reproduce](#reproduce) below and run
`/tmp/thai_bench/bin/python test/eval_oov_compare.py newmm nlpo3 attacut deepcut` plus
`… eval_oov_compare.py thapthim-LST20 --pred /tmp/pred_lst20`.

## Speed — pure tokenization throughput

Representative best-of-5 on the LST20 test text. Thapthim is measured through its Ruby↔Rust FFI
(`ruby test/benchmark_speed.rb`); the Python baselines through their native bindings.

| engine | char/s | vs thapthim |
|---|--:|--:|
| nlpo3 | ~3.8M | 1.5× faster |
| **thapthim** (either LM) | **~2.6M** | — |
| newmm | ~1.1M | 0.42× |
| attacut | ~95k | ~27× slower |
| deepcut | ~3.5k | ~740× slower |

Thapthim's two LMs run at the same speed. Throughput reflects a series of hot-path improvements: a
splitmix64 bigram-key hasher, precomputing candidate token ids in the Viterbi decode,
allocation-free grid candidates (the generic-lattice refactor dropped the per-candidate owned
`String`), a flat `byte → TCC-index` array that replaces the per-match hashmap probes in candidate
generation (also the grid-membership test, so no separate boundary set), and per-thread reuse of the
Viterbi DP scratch buffers — ~450k → ~2.6M char/s overall; see CHANGELOG.

All figures above are **single-threaded, per-call** — the only basis on which engines are
comparable (every baseline here is run single-threaded too). The thapthim row is the Ruby↔Rust FFI;
the **Python (PyO3) binding runs the identical engine and assets at ~3.0M char/s single-core**
(per-call, same LST20 text) — marginally ahead of the Ruby figure because PyO3's call overhead is
lower, not because the segmentation differs. Beyond single-core, the Python binding's
`segment_batch` reports a much higher number (~10M char/s on 8 cores) because it releases the GIL
and fans the batch across all cores with rayon; that is a **multicore deployment-throughput** figure,
not an engine-speed one, and is **not** comparable to the single-threaded numbers above (any
tokenizer can be batched/parallelized the same way). Use the per-call figure for model-vs-model
comparison; treat batch throughput as "what one machine can push using all cores."

## Syllable segmentation

`Thapthim.syllables` segments into orthographic syllables via a single syllable-LM Viterbi over the
TCC grid. Its syllable LM is trained on SSG (PyThaiNLP's `engine="ssg"`) applied per gold word, so
SSG is the natural baseline.

| metric | result |
|---|--:|
| agreement with SSG training target (per-word, boundary F1, LST20) | **0.9941** |
| speed (LST20 test, best-of-5, Ruby FFI) | **~3.5M char/s** (~23,600 sent/s) |
| speed (same text, Python/PyO3, single-core) | ~4.3M char/s |
| SSG speed (same corpus) | ~0.20M char/s (~1,400 sent/s) |

So syllable segmentation reproduces its SSG target near-perfectly while running **~17× faster than
SSG** — and faster than thapthim's own word segmentation (single Viterbi pass over the smaller
syllable dictionary, with none of the word path's OOV-run / entropy-merge post-processing).
(Against raw SSG-on-full-text the boundary F1 is 0.81, but that gap is purely a space/number
tokenization convention — thapthim keeps `" "` and numbers as standalone tokens — not a quality
difference.)

## Takeaways

- **Thapthim posts the top word-level F1 on 4 of 5 corpora** (lst20, vistec, tnhc, ws1000); deepcut
  wins its home corpus **best** (0.9659). Read this with the home-corpus caveat below: the four it
  tops include its own training corpus (lst20) and three on which the neural baselines run
  out-of-domain. The cleanest like-for-like is each tool on its home turf — there thapthim is
  competitive rather than ahead (on best: thapthim-BEST 0.950 ≈ attacut 0.945, below deepcut 0.966).
- **vs the neural models** (attacut, deepcut): Thapthim matches or beats them off-domain while
  being **~27× faster than attacut and ~740× faster than deepcut**. The neural models only pull
  ahead on **best**, the corpus they are trained on — and there the gated **thapthim-BEST** LM
  (0.9496) already edges attacut (0.9454) and trails only deepcut, at a fraction of the cost.
- **vs the dictionary tools** (nlpo3, newmm): nlpo3 is ~1.5× faster than Thapthim but ~14–24
  word-F1 points worse on every corpus; having no statistical model, they plateau well below both
  Thapthim and the neural engines.
- **Where it fits:** the combination is the unusual part — accuracy competitive with the neural
  models across domains, at dictionary-class throughput, with in-vocab recall the strongest here.
  The trade-off is OOV recall (~0.25 micro, the dictionary tier — see the OOV section), so on
  high-OOV text the neural models close in.

## Caveats

- **Cross-annotation-standard.** Each tool favors the corpus matching its training standard
  (thapthim-LST20 → LST20, thapthim-BEST/attacut/deepcut → BEST, nlpo3/newmm → LEXiTRON-style
  dictionary). The home-corpus advantage is real; read each tool both on its home turf and on the
  out-of-domain corpora (tnhc, vistec, ws1000).
- **Baselines are evaluated as shipped (BEST-trained).** The released `deepcut` and `attacut-sc`
  packages are fixed models trained on BEST, so in this table **only the BEST column is in-domain
  for them** — every other column is cross-domain and would likely be higher if they were retrained
  per corpus. This is corroborated by the UnifiedCut paper (Wen et al., 2024), whose Table 5 shows a
  BEST-trained DeepCut dropping to ORCHID 0.66 / TNHC 0.63 / Wisesight 0.81 cross-domain; our
  deepcut matches the paper on BEST (0.966 vs 0.963), confirming the harness. So thapthim's lead on
  the non-BEST corpora is partly its home advantage against the baselines' cross-domain handicap —
  the cleanest like-for-like is each tool on its home corpus (on BEST: thapthim-BEST 0.950 ≈ attacut
  0.945, below deepcut 0.966).
- **Metric averaging.** We report **micro** F1 (TP/FP/FN aggregated over the corpus). Some papers
  (e.g. AttaCut, UnifiedCut) report **macro** F1 (mean ± std of per-sentence F1), which runs a few
  tenths to ~1.5 points lower here and is not directly comparable to these numbers.
- **Held-out splits.** Thapthim's dictionary is decontaminated of the BEST test split, so
  thapthim-BEST's `best` score is a fair held-out number, not memorization.
- **Full test sets.** Every corpus is scored at full size (lst20 5,250 · best 27,834 · vistec 10,000
  · tnhc 4,403 · ws1000 993). For a quick smoke run, cap each corpus with `LIMIT=N`.

## Reproduce

```bash
# 1. baselines into a throwaway venv (NOT gem deps)
python3 -m venv /tmp/thai_bench
/tmp/thai_bench/bin/pip install "pythainlp[benchmarks]" attacut deepcut nlpo3 tensorflow

# 2. Thapthim predictions. The shipped LST20 LM needs no flags; the gated BEST and COMBINED LMs
#    need a build that embeds them (one binary can carry all three):
#      (cd ext/thapthim && cargo rustc --release --features best_lm,combined_lm --crate-type cdylib)
#      cp target/release/libthapthim.dylib lib/thapthim/thapthim.bundle
ruby test/dump_segmentation.rb /tmp/pred_lst20
THAPTHIM_LM=best     ruby test/dump_segmentation.rb /tmp/pred_best
THAPTHIM_LM=combined ruby test/dump_segmentation.rb /tmp/pred_combined

# 3. score every engine with the identical research metric
/tmp/thai_bench/bin/python test/benchmark_accuracy.py thapthim-LST20    --pred /tmp/pred_lst20
/tmp/thai_bench/bin/python test/benchmark_accuracy.py thapthim-BEST     --pred /tmp/pred_best
/tmp/thai_bench/bin/python test/benchmark_accuracy.py thapthim-COMBINED --pred /tmp/pred_combined
for e in nlpo3 newmm attacut deepcut; do
  /tmp/thai_bench/bin/python test/benchmark_accuracy.py "$e"
done

# 4. speed
ruby test/benchmark_speed.rb 5000 5
```
