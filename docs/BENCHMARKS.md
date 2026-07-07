# Thapthim Benchmarks

Thapthim vs the common Thai word-segmentation engines, scored with the research-standard metric (the
`pythainlp.benchmarks` reference used by the AttaCut/DeepCut papers; whitespace stripped before
scoring). **char-F1** = per-character boundary detection; **word-F1** = a word is correct iff *both*
boundaries match.

Last run: thapthim (all three LMs) re-measured 2026-06-29 on `main` @ 4f6b2df, post decode fixes
`274ec01`/`82caf74`; baselines from 2026-06-24. Apple M1 · pythainlp 5.3.4, attacut 1.0.6
(`attacut-sc`), deepcut 0.7.0.0 (TF 2.21), nlpo3 1.4.0.

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

Full test sets: lst20 5,250 · best 27,834 · vistec 10,000 · tnhc 4,403 · ws1000 993 sentences. LST20
ships; BEST/COMBINED are gated (same engine, different LM). All engines reconstructed every sentence
exactly.

## Word-level F1 (**bold** = best per corpus)

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

Among thapthim's LMs (engine identical, only the training corpus differs), each single-corpus LM wins
its home corpus; **COMBINED** is the best all-rounder — top word-F1 macro (0.865) of any engine here,
and never collapses. Shipped default is LST20 (highest home peak). The KN discount, swept 0.1–0.99 on
all three LMs, had no meaningful effect (argmax is near-invariant to a uniform shift), so `d = 0.75`
is kept.

## OOV recall — generalization to unknown words

Word-F1 is dominated by frequent in-dictionary words and hides how each engine handles unseen ones.
This stratifies **recall** by dictionary membership (SIGHAN-style), using one **shared OOV reference**
for every engine — Thapthim's shipped lexicon (141,548 words); a gold word is OOV iff absent from it,
so every engine is scored on the identical OOV set. Caps: lst20 5,250 · best 3,000 · vistec 3,000 ·
tnhc 4,403 · ws1000 993 (bounds deepcut runtime). thapthim re-measured 2026-06-29; baselines 2026-06-25.

### R_oov (**bold** = best per corpus)

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

**OOV recall is Thapthim's weakest dimension** — ~0.25 micro, the dictionary tier: ahead of nlpo3,
level with newmm, behind both neural (deepcut recovers 1.6× as many). A dictionary model can't invent
boundaries for words absent from its lexicon — the same design that gives it the best in-vocab recall
here (0.898, COMBINED 0.911). OOV is hard for all; even deepcut clears only 0.42.

**The LM corpus barely touches OOV recall** (all three LMs within 0.0002 micro): OOV merging is driven
by the LM-independent branching-entropy pass, not the bigram. The LM instead moves in-vocab
disambiguation. So the F1 lead survives weak OOV — candidates come from a broad union dictionary, OOV
rates are low (0.8–10%), and in-vocab recall dominates; OOV only bites on high-OOV corpora (vistec,
ws1000).

Reproduce: `ruby test/eval_oov.rb`, or the cross-model table via the dumps in
[Reproduce](#reproduce) + `test/eval_oov_compare.py`.

## Fair comparison — corpus-controlled

The tables above flatter Thapthim two ways unrelated to the *method*: the shipped dictionary is a
broad union (LST20 ∪ BEST ∪ PyThaiNLP), and the entropy merge is TNHC-tuned — while the neural
baselines are plain BEST-trained. The `·fair` builds strip Thapthim to one corpus: dictionary *and* LM
from a single corpus, merge off (`THAPTHIM_WORD_VOCAB` + `THAPTHIM_BE_THRESHOLD=0`); OOV reference
unchanged. Re-measured 2026-06-30 post decode-fixes (same code state as above); this lifted the fair
builds ~+0.01 macro off-domain, which strengthens the conclusion.

### Word-level F1 (single-corpus, no entropy; **bold** = best per corpus)

| corpus | LST20·fair | BEST·fair | attacut (BEST) | deepcut (BEST) |
|---|--:|--:|--:|--:|
| **lst20**  | **0.9522** | 0.8481 | 0.8532 | 0.8522 |
| **best**   | 0.8680 | 0.9492 | 0.9454 | **0.9659** |
| **vistec** | **0.8016** | 0.7979 | 0.7843 | 0.7971 |
| **tnhc**   | 0.7912 | **0.7932** | 0.7667 | 0.7764 |
| **ws1000** | **0.8281** | 0.8191 | 0.8261 | 0.8243 |
| **macro-avg** | **0.8482** | 0.8415 | 0.8351 | 0.8432 |

- **Home advantage dominates** — each single-corpus model wins its own corpus by ~+0.08–0.10; most
  cross-tool gaps are home-vs-away, not method quality.
- **Corpus-controlled, the deterministic method matches or slightly edges neural** — fair macro
  0.842–0.848 vs attacut 0.835 / deepcut 0.843; the better fair build wins all three no-one's-home
  corpora (vistec/tnhc/ws1000). Read as "competitive-to-ahead when controlled," since the neural
  baselines run cross-domain there.
- **The shipped cross-domain lead is the union dictionary, not the algorithm** — union+entropy adds
  ~+0.003–0.016 off-home and is flat-to-−0.002 on-home. A real, cheap, deterministic advantage — but
  separate from the Viterbi method.

### OOV recall (single-corpus, no entropy)

| corpus | LST20·fair | BEST·fair | attacut | deepcut |
|---|--:|--:|--:|--:|
| **lst20**  | 0.165 | 0.162 | 0.328 | 0.351 |
| **best**   | 0.075 | 0.080 | 0.383 | 0.526 |
| **vistec** | 0.267 | 0.266 | 0.266 | 0.423 |
| **tnhc**   | 0.263 | 0.243 | 0.312 | 0.374 |
| **ws1000** | 0.437 | 0.430 | 0.518 | 0.542 |

**OOV recall is invariant to the training corpus** — single-corpus builds land within ~0.005–0.02 of
each other and of shipped: a word OOV to the union is OOV to either single dict, and recall on it is
fixed by the syllabify fallback, not the dict or LM. So the gap to neural is structural to the
dictionary-lattice approach, not a corpus artifact.

**The TNHC-tuned merge isn't an unfair edge elsewhere.** Re-running each fair build merge-off /
TNHC-table / corpus-native-table: word-F1 is neutral except on TNHC (±0.002 elsewhere); the merge's
one win is TNHC text (+0.005–0.008 F1), where the TNHC table even beats the native one (a
literary-domain effect, not leakage); native entropy trades a little off-domain OOV recall for
precision. So the shipped tuning carries no hidden advantage on the four non-TNHC corpora. *(Pre-06-27
figures; conclusion qualitative.)*

## Speed — pure tokenization throughput

Best-of-5 on LST20 test text; thapthim via its Ruby↔Rust FFI (`ruby test/benchmark_speed.rb`),
baselines via native bindings.

| engine | char/s | vs thapthim |
|---|--:|--:|
| nlpo3 | ~3.8M | 1.3× faster |
| **thapthim** (either LM) | **~2.9M** | — |
| newmm | ~1.1M | 0.38× |
| attacut | ~95k | ~30× slower |
| deepcut | ~3.5k | ~820× slower |

Both LMs run at the same speed; throughput comes from output-identical hot-path wins (see CHANGELOG).
All figures are **single-threaded, per-call** (the only comparable basis). The Python/PyO3 binding
runs the same engine at ~3.2M char/s; `word_segment_batch` reaches ~11M char/s on 8 cores — a
multicore-deployment figure, not engine speed. The first call in a fresh process pays a one-time
~0.2 s bootstrap (LM deserialize, TCC regex compile, trie build) — per-process, amortized to nothing
in any server/batch workload.

## Syllable segmentation

`syllable_segment` runs one syllable-LM Viterbi over the TCC grid (same engine, syllable dict + LM).
Its LM is trained on SSG (PyThaiNLP `engine="ssg"`) per gold word, so SSG is the natural baseline.

| metric | result |
|---|--:|
| agreement with SSG training target (per-word, boundary F1, LST20) | **0.9941** |

It reproduces the SSG target near-perfectly. (Raw SSG-on-full-text F1 is 0.81, but that gap is a
space/number tokenization convention — thapthim keeps `" "` and numbers as standalone tokens — not a
quality difference.) Method: minimum-cluster + n-gram Viterbi, an established recipe (Jucksriporn &
Sornil 2011; cf. Aroonmanakun 2002) — a utility, not a contribution.

## Part-of-speech tagging

`pos_tag` is a first-order (bigram) HMM over the 16-tag LST20 tagset — a standalone dense Viterbi,
**not** the segmentation lattice ([ARCHITECTURE](ARCHITECTURE.md#part-of-speech-tagging)). Evaluated
on LST20 test with **gold word tokens** (so a segmentation error can't contaminate the number), split
by known vs OOV word.

| metric | result |
|---|--:|
| overall token accuracy (LST20 test, gold tokens, spaces excluded) | **92.90%** |
| known-word accuracy | 93.37% |
| OOV-word accuracy | 70.23% |
| OOV share of scored tokens | 2.03% |

Full set: 5,250 sentences · 207,278 tokens, of which **174,074 are scored** — the LST20 space token
`_`→PU is ~16% of tokens and trivially 100% correct (space is PU by formatting, not a tagging
decision), so it is excluded from scoring (still fed to the tagger as context). Decode ≈ 700K tokens/s
single-threaded (Ruby FFI). A bigram HMM is the textbook baseline, so this is a utility, not a
contribution: ~93% is in the expected band (LST20 neural taggers ~96–97%; the gap is the independence
assumption, CRF territory). Top confusions are content-word ambiguities a bigram can't resolve (NN↔NU,
AV↔VV, CL↔NN). OOV is only ~2% of scored tokens in-domain; a Witten-Bell unknown model plus a few
orthographic rules (digit→`NU`, symbol→`PU`, foreign/karan/affix→`NN`) handle it, and cross-domain
accuracy will be lower. Reproduce:
`ruby test/eval_pos.rb datasets/LST20_full_train datasets/LST20_full_test`.

## Takeaways

- **Top word-F1 on 4 of 5 corpora** (deepcut wins best, 0.966) — but those four include thapthim's
  home corpus and three where the neural baselines run out-of-domain. Like-for-like on home turf,
  thapthim is competitive, not ahead (best: thapthim-BEST 0.950 ≈ attacut 0.945 < deepcut 0.966).
- **vs neural:** competitive off-domain at ~30× attacut / ~820× deepcut speed; they lead on best (their
  training corpus).
- **vs dictionary tools:** ~1.3× slower than nlpo3 but +14–24 word-F1 points on every corpus.
- **Where it fits:** neural-competitive in-domain (corpus-controlled) at dictionary-class speed, best
  in-vocab recall here; the trade-off is OOV recall (~0.25). Established architecture (Kawtrakul &
  Thumkanon 1997) — the contribution is the controlled finding, not the method.

## Caveats

- **Cross-annotation-standard** — each tool favors its training standard; the home-corpus advantage is
  real, so read each both on home turf and out-of-domain (tnhc, vistec, ws1000).
- **Baselines evaluated as shipped (BEST-trained)** — only the best column is in-domain for
  deepcut/attacut; others are cross-domain and would likely be higher retrained. Corroborated by
  UnifiedCut (Wen et al., 2024), and our deepcut matches the paper on best (0.966 vs 0.963).
- **Micro F1** reported (aggregated TP/FP/FN); macro (some papers) runs a few tenths to ~1.5 pts lower,
  not directly comparable.
- **Held-out** — the dictionary is decontaminated of the BEST test split, so thapthim-BEST's best score
  is not memorization.

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
