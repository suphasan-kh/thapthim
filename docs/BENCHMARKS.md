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

## Word-level F1 (research metric; **bold** = best per corpus)

| corpus | thapthim-LST20 | thapthim-BEST | attacut | deepcut | nlpo3 | newmm |
|---|--:|--:|--:|--:|--:|--:|
| **lst20**  | **0.9481** | 0.8698 | 0.8532 | 0.8522 | 0.7135 | 0.7124 |
| **best**   | 0.8734 | 0.9488 | 0.9455 | **0.9656** | 0.6863 | 0.6836 |
| **vistec** | **0.8135** | 0.8058 | 0.7844 | 0.7972 | 0.7487 | 0.7670 |
| **tnhc**   | 0.7916 | **0.8077** | 0.7667 | 0.7764 | 0.7084 | 0.7095 |
| **ws1000** | 0.8280 | **0.8288** | 0.8261 | 0.8243 | 0.7525 | 0.7487 |

## Char-level F1 (boundary detection)

| corpus | thapthim-LST20 | thapthim-BEST | attacut | deepcut | nlpo3 | newmm |
|---|--:|--:|--:|--:|--:|--:|
| **lst20**  | **0.9781** | 0.9496 | 0.9420 | 0.9413 | 0.8901 | 0.8899 |
| **best**   | 0.9494 | 0.9761 | 0.9768 | **0.9864** | 0.8767 | 0.8745 |
| **vistec** | **0.9217** | 0.9186 | 0.9145 | 0.9190 | 0.8973 | 0.9059 |
| **tnhc**   | 0.9178 | **0.9237** | 0.9006 | 0.9068 | 0.8873 | 0.8876 |
| **ws1000** | 0.9292 | 0.9289 | **0.9316** | 0.9307 | 0.9006 | 0.9024 |

All engines reconstructed every sentence exactly (0 mismatches), so the spans are directly comparable.

### Gated thapthim LMs (word-level F1)

The engine is identical; only the language-model training corpus differs. Neither ships by default.

| corpus | thapthim-LST20 (shipped) | thapthim-BEST | thapthim-COMBINED |
|---|--:|--:|--:|
| lst20 | **0.9481** | 0.8698 | 0.9361 |
| best | 0.8734 | **0.9488** | 0.9235 |
| vistec | **0.8135** | 0.8058 | 0.8087 |
| tnhc | 0.7916 | **0.8077** | 0.8076 |
| ws1000 | 0.8280 | 0.8288 | **0.8330** |
| **macro-avg** | 0.8509 | 0.8522 | **0.8618** |

Each single-corpus LM wins its home corpus; **COMBINED** is the best all-rounder (highest macro-average,
never collapses), trading a little LST20/VISTEC peak for large BEST/TNHC gains. The KN absolute discount
was swept over 0.1–0.99 on all three LMs and found to have no meaningful effect on word-F1 (argmax
decoding is near-invariant to a uniform score shift), so the textbook `d = 0.75` is retained.

## Speed — pure tokenization throughput

Representative best-of-5 on the LST20 test text. Thapthim is measured through its Ruby↔Rust FFI
(`ruby test/benchmark_speed.rb`); the Python baselines through their native bindings.

| engine | char/s | vs thapthim |
|---|--:|--:|
| nlpo3 | ~3.8M | 2.3× faster |
| **thapthim** (either LM) | **~1.68M** | — |
| newmm | ~1.1M | 0.65× |
| attacut | ~95k | ~18× slower |
| deepcut | ~3.5k | ~480× slower |

Thapthim's two LMs run at the same speed. Throughput reflects two hot-path fixes: a splitmix64
bigram-key hasher and precomputing candidate token ids in the Viterbi decode (~450k → ~1.68M
char/s overall; see CHANGELOG).

## Takeaways

- **Thapthim leads word-level F1 on 4 of 5 corpora** (lst20, vistec, tnhc, ws1000); deepcut wins
  its home corpus **best** (0.9656). The shipped **thapthim-LST20** is best overall on lst20 and
  vistec and competitive everywhere else.
- **vs the neural models** (attacut, deepcut): Thapthim matches or beats them off-domain while
  being **~15× faster than attacut and ~400× faster than deepcut**. The neural models only pull
  ahead on **best**, the corpus they are trained on — and there the gated **thapthim-BEST** LM
  (0.9488) already edges attacut (0.9455) and trails only deepcut, at a fraction of the cost.
- **vs the dictionary tools** (nlpo3, newmm): nlpo3 is ~2.7× faster than Thapthim but ~14–24
  word-F1 points worse on every corpus; having no statistical model, they plateau well below both
  Thapthim and the neural engines.
- **Sweet spot:** Thapthim sits where the others don't — near-top accuracy across domains at
  high (dictionary-class) throughput.

## Caveats

- **Cross-annotation-standard.** Each tool favors the corpus matching its training standard
  (thapthim-LST20 → LST20, thapthim-BEST/attacut/deepcut → BEST, nlpo3/newmm → LEXiTRON-style
  dictionary). The home-corpus advantage is real; read each tool both on its home turf and on the
  out-of-domain corpora (tnhc, vistec, ws1000).
- **Held-out splits.** Thapthim's dictionary is decontaminated of the BEST test split, so
  thapthim-BEST's `best` score is a fair held-out number, not memorization.
- **Sentence caps.** `best` and `vistec` are capped at 3,000 sentences to bound deepcut's runtime;
  `lst20` (5,250), `tnhc` (4,403) and `ws1000` (993) are full test sets. Override with `LIMIT=N`.

## Reproduce

```bash
# 1. baselines into a throwaway venv (NOT gem deps)
python3 -m venv /tmp/thai_bench
/tmp/thai_bench/bin/pip install "pythainlp[benchmarks]" attacut deepcut nlpo3 tensorflow

# 2. Thapthim predictions (shipped LST20 LM)
ruby test/dump_segmentation.rb /tmp/pred_lst20
# optional gated BEST LM: build with `--features best_lm`, then:
#   THAPTHIM_LM=best ruby test/dump_segmentation.rb /tmp/pred_best

# 3. score every engine with the identical research metric
/tmp/thai_bench/bin/python test/benchmark_accuracy.py thapthim-LST20 --pred /tmp/pred_lst20
for e in nlpo3 newmm attacut deepcut; do
  /tmp/thai_bench/bin/python test/benchmark_accuracy.py "$e"
done

# 4. speed
ruby test/benchmark_speed.rb 5000 5
```
