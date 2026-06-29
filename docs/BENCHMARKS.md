# Thapthim Benchmarks

Accuracy and speed of Thapthim against the common Thai word-segmentation engines, scored with
the **research-standard metric** used by the AttaCut and DeepCut papers — the
`pythainlp.benchmarks` reference implementation. Two F1 levels are reported (whitespace is
stripped before scoring, per the reference preprocessing):

- **char-level F1** — word-boundary detection: every character is labelled start-of-word or not.
- **word-level F1** — a predicted word is correct iff **both** of its boundaries match the gold word.

Last run: thapthim columns (all three LMs) re-measured 2026-06-29 on `main` @ 4f6b2df, after the
`274ec01` single-consonant-fold and `82caf74` western-anchor decode fixes (+F1 all corpora); the
non-thapthim baselines are unchanged from 2026-06-24 (those are other tools, unaffected). Apple M1
(8 cores) · pythainlp 5.3.4, attacut 1.0.6 (`attacut-sc`), deepcut 0.7.0.0 (TensorFlow 2.21),
nlpo3 1.4.0.

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
| **lst20**  | **0.9503** | 0.8711 | 0.9379 | 0.8532 | 0.8522 | 0.7135 | 0.7124 |
| **best**   | 0.8749 | 0.9510 | 0.9255 | 0.9454 | **0.9659** | 0.6870 | 0.6839 |
| **vistec** | **0.8175** | 0.8106 | 0.8131 | 0.7843 | 0.7971 | 0.7480 | 0.7667 |
| **tnhc**   | 0.7953 | **0.8111** | 0.8110 | 0.7667 | 0.7764 | 0.7084 | 0.7095 |
| **ws1000** | 0.8309 | 0.8312 | **0.8364** | 0.8261 | 0.8243 | 0.7525 | 0.7487 |
| **macro-avg** | 0.8538 | 0.8550 | **0.8648** | 0.8351 | 0.8432 | 0.7219 | 0.7242 |

## Char-level F1 (boundary detection)

| corpus | LST20 | BEST | COMBINED | attacut | deepcut | nlpo3 | newmm |
|---|--:|--:|--:|--:|--:|--:|--:|
| **lst20**  | **0.9793** | 0.9505 | 0.9748 | 0.9420 | 0.9413 | 0.8901 | 0.8899 |
| **best**   | 0.9501 | 0.9772 | 0.9680 | 0.9771 | **0.9865** | 0.8770 | 0.8747 |
| **vistec** | **0.9240** | 0.9211 | 0.9224 | 0.9146 | 0.9192 | 0.8970 | 0.9060 |
| **tnhc**   | 0.9200 | **0.9258** | 0.9258 | 0.9006 | 0.9068 | 0.8873 | 0.8876 |
| **ws1000** | 0.9314 | 0.9308 | **0.9332** | 0.9316 | 0.9307 | 0.9006 | 0.9024 |

Among thapthim's LMs (engine identical, only the training corpus differs): each single-corpus LM wins
its home corpus, while **COMBINED** is the best all-rounder — highest word-F1 macro-average (0.865)
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
tnhc 4,403 · ws1000 993) to bound deepcut's runtime. Last run: thapthim columns re-measured
2026-06-29 (`main` @ 4f6b2df, post-decode-fix); baselines unchanged from 2026-06-25.

### R_oov — recall on out-of-vocabulary words (**bold** = best per corpus)

| corpus | OOV% | LST20 | BEST | COMBINED | attacut | deepcut | nlpo3 | newmm |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| **lst20**  | 1.6%  | 0.1629 | 0.1629 | 0.1629 | 0.3282 | **0.3506** | 0.1494 | 0.1486 |
| **best**   | 0.8%  | 0.0738 | 0.0738 | 0.0738 | 0.3833 | **0.5262** | 0.0905 | 0.0452 |
| **vistec** | 10.0% | 0.2704 | 0.2703 | 0.2703 | 0.2661 | **0.4228** | 0.1858 | 0.2782 |
| **tnhc**   | 5.0%  | 0.2709 | 0.2710 | 0.2703 | 0.3121 | **0.3735** | 0.2729 | 0.2810 |
| **ws1000** | 10.4% | 0.4293 | 0.4283 | 0.4283 | 0.5179 | **0.5415** | 0.4191 | 0.3863 |
| **micro-avg** | 5.1% | 0.2657 | 0.2656 | 0.2655 | 0.2992 | **0.4157** | 0.2128 | 0.2682 |

### R_iv — recall on in-vocabulary words (micro-avg)

| | LST20 | BEST | COMBINED | attacut | deepcut | nlpo3 | newmm |
|---|--:|--:|--:|--:|--:|--:|--:|
| **R_iv** | 0.8979 | 0.8981 | **0.9109** | 0.8554 | 0.8601 | 0.7280 | 0.7262 |

**OOV recall is Thapthim's weakest dimension.** At ~0.25 micro it sits in the *dictionary tier* —
slightly ahead of nlpo3 (the entropy merge earns a real edge), level with newmm, behind both neural
models (~13% below attacut, ~57% below deepcut, which recovers 1.6× as many unknown words). The
dictionary-model trade-off: the approach that gives Thapthim the **best in-vocab recall here** (0.898,
0.911 combined) can't invent boundaries for words absent from its lexicon. OOV is hard for everyone —
even deepcut clears only 0.42.

**The LM corpus barely touches OOV recall** — all three LMs land within 0.0002 micro (per-corpus
≤ 0.001), because OOV merging is driven by the LM-independent branching-entropy post-pass, not the
bigram LM. The LM instead moves **in-vocab** disambiguation (each single-corpus LM peaks R_iv on its
home corpus; combined is the best all-rounder, micro R_iv 0.898→0.911). This is why the headline F1
lead survives weak OOV recall: candidate words come from a broad *union* dictionary and OOV handling
is domain-general, OOV rates are low (0.8–10%), and in-vocab recall dominates — OOV only bites on the
high-OOV corpora (vistec, ws1000), where deepcut closes the gap.

Reproduce: `ruby test/eval_oov.rb` (Thapthim alone, shipped LM) or, for the cross-model table,
generate the dumps as in [Reproduce](#reproduce) below and run
`/tmp/thai_bench/bin/python test/eval_oov_compare.py newmm nlpo3 attacut deepcut` plus
`… eval_oov_compare.py thapthim-LST20 --pred /tmp/pred_lst20`.

## Fair comparison — corpus-controlled (single-corpus dictionary, no entropy merge)

The tables above flatter Thapthim in two ways that aren't about the segmentation *method*: the
shipped dictionary is a broad **union** (LST20 ∪ BEST ∪ PyThaiNLP), and the branching-entropy merge
is tuned on TNHC — whereas the neural baselines are plain BEST-trained. To compare like-for-like, we
strip Thapthim to a **single-corpus build**: dictionary *and* n-gram LM from one corpus only, and the
branching-entropy merge **off**. `thapthim-LST20·fair` and `thapthim-BEST·fair` below are exactly
that — the word vocabulary is the unique tokens of that corpus's train split, the LM is that corpus's
LM (`THAPTHIM_LM`), the merge is disabled (`THAPTHIM_BE_THRESHOLD=0`), and the OOV reference stays the
shared union lexicon as above. (The single-corpus dictionary is supplied on the shipped engine via
`THAPTHIM_WORD_VOCAB=<file>`; predictions are dumped with `test/dump_segmentation.rb` and scored with
the same `test/benchmark_accuracy.py` research metric as every other engine — no separate branch.)

> **Re-measured 2026-06-30 on `main`** (post `274ec01`/`82caf74` decode fixes), via the shipped engine
> with `THAPTHIM_WORD_VOCAB` (single-corpus dictionary) and `THAPTHIM_BE_THRESHOLD=0` (merge off) — so
> the word-F1 and OOV tables below now share the **same code state** as the tables above (word-F1 at
> full test-set sizes; OOV at the capped sizes of the OOV section). Re-measuring lifted the fair builds
> by ~+0.01 macro, concentrated on the cross-domain corpora, which **strengthens** the corpus-controlled
> conclusion rather than weakening it. The *merge A/B* subsection further down still carries its pre-fix
> figures; its conclusion is qualitative and unaffected.

### Word-level F1 (single-corpus, no entropy; **bold** = best per corpus)

| corpus | LST20·fair | BEST·fair | attacut (BEST) | deepcut (BEST) |
|---|--:|--:|--:|--:|
| **lst20**  | **0.9522** | 0.8481 | 0.8532 | 0.8522 |
| **best**   | 0.8680 | 0.9492 | 0.9454 | **0.9659** |
| **vistec** | **0.8016** | 0.7979 | 0.7843 | 0.7971 |
| **tnhc**   | 0.7912 | **0.7932** | 0.7667 | 0.7764 |
| **ws1000** | **0.8281** | 0.8191 | 0.8261 | 0.8243 |
| **macro-avg** | **0.8482** | 0.8415 | 0.8351 | 0.8432 |

- **Home advantage dominates.** Each single-corpus model wins its own corpus by a wide margin —
  LST20·fair on lst20 (0.952 vs the BEST model's 0.848, **+0.10**) and BEST·fair on best (0.949 vs
  0.868, **+0.08**). Most apparent cross-tool gaps are home-vs-away, not method quality.
- **Controlled for corpus, the deterministic method matches or slightly edges the neural models.**
  Fair macro-avg 0.842–0.848 vs attacut 0.835 / deepcut 0.843 — LST20·fair tops both; BEST·fair beats
  attacut and trails deepcut by 0.002. On the three corpora that are *no one's* home turf (vistec,
  tnhc, ws1000), the better fair build wins all three, by margins from +0.002 (ws1000) to +0.017
  (tnhc) — though the neural baselines run cross-domain there (BEST-trained; see Caveats), so read
  this as "competitive-to-ahead when corpus-controlled," not a clean method win.
- **The shipped system's cross-domain lead is the union dictionary, not the algorithm.** Re-adding
  union coverage + entropy lifts each fair build by **~+0.003–0.016 off-home** (most on vistec/best,
  where the union carries the *other* domains' vocabulary) and is **flat-to-slightly-negative on-home**
  (−0.002 on lst20, where extra candidates only add noise the bigram must resist). A broad-coverage
  dictionary is a real, cheap, deterministic advantage off-domain — but a *separate* one from the
  Viterbi method.

### OOV recall (single-corpus, no entropy)

| corpus | LST20·fair | BEST·fair | attacut | deepcut |
|---|--:|--:|--:|--:|
| **lst20**  | 0.165 | 0.162 | 0.328 | 0.351 |
| **best**   | 0.075 | 0.080 | 0.383 | 0.526 |
| **vistec** | 0.267 | 0.266 | 0.266 | 0.423 |
| **tnhc**   | 0.263 | 0.243 | 0.312 | 0.374 |
| **ws1000** | 0.437 | 0.430 | 0.518 | 0.542 |

**OOV recall is invariant to the training corpus.** LST20-only and BEST-only builds land within
~0.005–0.02 of each other (largest on tnhc) and within noise of shipped — a word OOV to the union is
OOV to either single-corpus dict too, and recall on it is fixed by the syllabify fallback, *not* the
dictionary or LM. Extending "the LM barely touches OOV recall": **neither dict nor LM moves it**, so
the gap to neural is structural to the dictionary-lattice approach, not a corpus artifact.

**The TNHC-tuned entropy merge is not an unfair advantage on the other corpora.** We re-ran each fair
build three ways — merge off / on-with-shipped-TNHC-table / on-with-a-corpus-native-table (the table
is unsupervised, so any corpus can train it). (a) word-F1 is **neutral except on TNHC** (±0.002 on the
other four); (b) the merge's one real win is TNHC text (+0.005–0.008 F1, ~+0.02 R_oov), and the **TNHC
table beats the corpus-native table there** (BEST·fair 0.7876 vs 0.7865) — a domain effect of TNHC's
literary text, not eval-set leakage into a tuned knob; (c) native entropy buys a little off-domain OOV
recall but trades precision (LST20·fair vistec R_oov 0.253→0.271, F1 flat). So the shipped TNHC tuning
carries no hidden advantage on the four non-TNHC corpora. *(These figures predate the 06-27 fixes;
the conclusion is qualitative.)*

## Speed — pure tokenization throughput

Representative best-of-5 on the LST20 test text. Thapthim is measured through its Ruby↔Rust FFI
(`ruby test/benchmark_speed.rb`); the Python baselines through their native bindings.

| engine | char/s | vs thapthim |
|---|--:|--:|
| nlpo3 | ~3.8M | 1.3× faster |
| **thapthim** (either LM) | **~2.9M** | — |
| newmm | ~1.1M | 0.38× |
| attacut | ~95k | ~30× slower |
| deepcut | ~3.5k | ~820× slower |

Both LMs run at the same speed. Throughput comes from output-identical hot-path wins (splitmix64
bigram hasher, token-id contexts instead of string hashing, allocation-free candidates, a flat
`byte→TCC-index` array, dense Viterbi predecessor buckets + counting sort, per-thread scratch reuse)
— from ~450k char/s originally to the figures above. Details in CHANGELOG.

All figures are **single-threaded, per-call** — the only comparable basis (every baseline is
single-threaded too). The thapthim row is the Ruby↔Rust FFI; the **Python/PyO3 binding runs the
identical engine at ~3.2M char/s** (lower call overhead, not different segmentation). The Python
`word_segment_batch` reports ~11M char/s on 8 cores by releasing the GIL and fanning across cores —
a **multicore deployment** figure, not engine speed, and not comparable to the per-call numbers.

**Cold start.** The numbers are *sustained* per-call throughput. Separately, the first call in a fresh
process pays a one-time **~0.2 s bootstrap** (deserialize the LM, compile the TCC regex, build the
tries) — per-process, not per-call; relevant only to short-lived invocations (a CLI that segments one
string and exits), amortized to nothing in any server/batch workload.

## Syllable segmentation

`Thapthim.syllable_segment` segments into orthographic syllables via a single syllable-LM Viterbi over the
TCC grid. Its syllable LM is trained on SSG (PyThaiNLP's `engine="ssg"`) applied per gold word, so
SSG is the natural baseline.

| metric | result |
|---|--:|
| agreement with SSG training target (per-word, boundary F1, LST20) | **0.9941** |
| speed (LST20 test, best-of-5, Ruby FFI) | **~4.0M char/s** (~26,900 sent/s) |
| speed (same text, Python/PyO3, single-core) | ~4.9M char/s |
| SSG speed (`engine="ssg"`, same corpus) | ~0.20M char/s (~1,340 sent/s) |
| dict speed (`engine="dict"`, same corpus) | ~0.48M char/s (~3,250 sent/s) |

So syllable segmentation reproduces its SSG target near-perfectly while running **~17× faster than
SSG** and **~7× faster than PyThaiNLP's `engine="dict"`** syllable tokenizer — and faster than
thapthim's own word pass (one Viterbi over the smaller syllable dict, no OOV/entropy post-processing).
(Against raw SSG-on-full-text the boundary F1 is 0.81, but that gap is a space/number tokenization
convention — thapthim keeps `" "` and numbers as standalone tokens — not a quality difference.)

## Takeaways

- **Top word-F1 on 4 of 5 corpora** (lst20, vistec, tnhc, ws1000); deepcut wins **best** (0.9659).
  Read with the home-corpus caveat: those four include thapthim's own training corpus (lst20) plus
  three where the neural baselines run out-of-domain. Cleanest like-for-like is each tool on home turf
  — there thapthim is competitive, not ahead (best: thapthim-BEST 0.950 ≈ attacut 0.945, < deepcut 0.966).
- **vs neural:** matches/beats them off-domain at **~30× attacut / ~820× deepcut** speed; they pull
  ahead only on **best** (their training corpus), where gated thapthim-BEST (0.9496) already edges
  attacut and trails only deepcut, at a fraction of the cost.
- **vs dictionary tools:** nlpo3 is ~1.3× faster but ~14–24 word-F1 points worse on every corpus —
  with no statistical model they plateau well below thapthim and the neural engines.
- **Where it fits:** accuracy competitive with neural across domains, at dictionary-class throughput,
  strongest in-vocab recall here. Trade-off is OOV recall (~0.25 micro), so high-OOV text narrows the gap.

## Caveats

- **Cross-annotation-standard.** Each tool favors its training standard (thapthim-LST20 → LST20,
  thapthim-BEST/attacut/deepcut → BEST, nlpo3/newmm → LEXiTRON-style dict). The home-corpus advantage
  is real — read each tool both on home turf and out-of-domain (tnhc, vistec, ws1000).
- **Baselines are evaluated as shipped (BEST-trained).** `deepcut` and `attacut-sc` are fixed
  BEST-trained models, so **only the BEST column is in-domain** for them; every other is cross-domain
  and would likely be higher if retrained. Corroborated by UnifiedCut (Wen et al., 2024): its Table 5
  shows BEST-trained DeepCut dropping cross-domain (ORCHID 0.66 / TNHC 0.63 / Wisesight 0.81), and our
  deepcut matches the paper on BEST (0.966 vs 0.963), confirming the harness. So thapthim's lead on
  non-BEST corpora is partly home advantage vs the baselines' cross-domain handicap.
- **Metric averaging.** We report **micro** F1 (aggregated TP/FP/FN). Some papers (AttaCut, UnifiedCut)
  report **macro** F1 (mean per-sentence), a few tenths to ~1.5 points lower here and not directly
  comparable.
- **Held-out splits.** Thapthim's dictionary is decontaminated of the BEST test split, so
  thapthim-BEST's `best` score is a fair held-out number, not memorization.
- **Full test sets** (lst20 5,250 · best 27,834 · vistec 10,000 · tnhc 4,403 · ws1000 993); `LIMIT=N`
  for a smoke run.

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

# 5. fair (corpus-controlled) tables: single-corpus dictionary + that corpus's LM, merge OFF.
#    Build the single-corpus vocab, then dump with THAPTHIM_WORD_VOCAB + THAPTHIM_BE_THRESHOLD=0.
ruby -rjson -e 'set={}; File.foreach("datasets/LST20_train_cleaned.jsonl"){|l| JSON.parse(l).each{|t| set[t]=1}}; File.open("/tmp/lst20_vocab.txt","w"){|f| set.each_key{|w| f.puts w}}'
THAPTHIM_WORD_VOCAB=/tmp/lst20_vocab.txt THAPTHIM_BE_THRESHOLD=0 LIMIT=1000000 ruby test/dump_segmentation.rb /tmp/pred_fair_lst20
LIMIT=1000000 /tmp/thai_bench/bin/python test/benchmark_accuracy.py thapthim-LST20 --pred /tmp/pred_fair_lst20
#    BEST·fair: same pattern with the BEST-train vocab, the best_lm build (step 2), and THAPTHIM_LM=best.
```
