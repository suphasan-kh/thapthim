# Methodology & Resources

Everything behind the numbers in [BENCHMARKS.md](BENCHMARKS.md): **what** was measured, **how**,
**with which tool**, **on what machine**, and **what each corpus actually looks like**. If a figure
appears in BENCHMARKS.md, this file tells you how it was produced and how to reproduce it. Nothing
here is trained or tuned — it is purely the measurement apparatus.

- [Measurement machine & software](#measurement-machine--software)
- [Corpora — what each one looks like](#corpora--what-each-one-looks-like)
- [Corpus file format](#corpus-file-format)
- [How scores are scored (metrics)](#how-scores-are-scored-metrics)
- [Accuracy — the two-stage pipeline](#accuracy--the-two-stage-pipeline)
- [Speed — how throughput is measured](#speed--how-throughput-is-measured)
- [OOV recall — how generalization is measured](#oov-recall--how-generalization-is-measured)
- [POS tagging — how accuracy is measured](#pos-tagging--how-accuracy-is-measured)
- [Spelling correction — how it is measured](#spelling-correction--how-it-is-measured)
- [Syllable segmentation — how it is measured](#syllable-segmentation--how-it-is-measured)
- [The shipped assets (in-repo resources)](#the-shipped-assets-in-repo-resources)

---

## Measurement machine & software

Every published number was produced on one machine, on a single code state, so all engines are
internally comparable.

| | |
|---|---|
| **Machine** | Apple MacBook Air (`MacBookAir10,1`) |
| **CPU** | Apple M1 — 8 cores (4 performance + 4 efficiency) |
| **RAM** | 8 GB |
| **OS** | macOS 26.5 (build 25F71) |
| **Ruby** | 3.3.0 (arm64-darwin) — drives Thapthim via its Ruby↔Rust FFI |
| **Rust** | release build (`cargo rustc --release --crate-type cdylib`) copied to `lib/thapthim/thapthim.bundle` |

**Baseline library versions** (installed into a throwaway venv, *not* gem dependencies):

| library | version | note |
|---|---|---|
| pythainlp | 5.3.4 | supplies both the `newmm`/`nlpo3` tokenizers **and** the reference scorer (`pythainlp.benchmarks`) |
| attacut | 1.0.6 | model `attacut-sc` |
| deepcut | 0.7.0.0 | on TensorFlow 2.21 |
| nlpo3 | 1.4.0 | Rust `newmm` |

Single-threaded, per-call is the only cross-engine-comparable basis, so that is what every timed
number uses (see [Speed](#speed--how-throughput-is-measured)).

---

## Corpora — what each one looks like

Six evaluation corpora. All are Thai word-segmentation gold data; they differ by **domain**,
**annotation standard**, and **size**. Counts below are measured directly from the files in
`datasets/` (test splits).

| corpus | domain / register | file | sentences (full) | gold tokens | chars | avg tok/sent | OOV %¹ | scoring cap² |
|---|---|---|--:|--:|--:|--:|--:|--:|
| **LST20** | news / mixed contemporary | `LST20_test_cleaned.jsonl` | 5,250 | 207,278 | 780,084 | 39.5 | 1.6% | 5,250 (full) |
| **BEST** | encyclopedic / literary / news | `BEST_test_cleaned.jsonl` | 27,834 | 987,773 | 3,940,309 | 35.5 | 0.8% | 3,000 |
| **VISTEC** | social media (Wongnai) | `VISTEC_test.jsonl` | 10,000 | 800,746 | 2,795,171 | 80.1 | 10.0% | 3,000 |
| **TNHC** | classical Thai literature | `tnhc_test.jsonl` | 4,403 | 141,004 | 504,867 | 32.0 | 5.0% | 4,403 (full) |
| **ws1000** | social / web (Wisesight-style) | `ws1000.jsonl` | 993 | 22,738 | 75,135 | 22.9 | 10.4% | 993 (full) |
| **ORCHID** | 1980s academic-conference papers | `orchid_test.jsonl` | 23,170 | 342,642 | 1,485,701 | 14.8 | 9.9% | 3,000 |

¹ **OOV %** = share of gold tokens absent from Thapthim's shipped 141,548-word lexicon (the shared OOV
reference — see [OOV recall](#oov-recall--how-generalization-is-measured)). This is a property of the
*corpus vs the dictionary*, not of any model.

² **Scoring cap** = how many sentences the cross-engine harness actually scores. LST20 / TNHC / ws1000
run in full; BEST / VISTEC / ORCHID are capped at 3,000 to bound deepcut's (very slow) runtime.
Predictions and gold are read under the identical cap so they line up. The caps are defined once in
[`test/benchmark_accuracy.py`](../test/benchmark_accuracy.py) and mirrored in
[`test/dump_segmentation.rb`](../test/dump_segmentation.rb).

### What each corpus is for

- **LST20** — the **shipped model's home corpus** (dictionary + language model are LST20-trained).
  Contemporary news text; the LST20 space token is annotated explicitly. Highest home-corpus score,
  so it is the shipped default.
- **BEST** — NECTEC's *Benchmark for Enhancing the Standard*. Used two ways: a deduplicated,
  shuffled **80:20 split (seed=42)** provides the `_train` (dictionary source) and a **held-out**
  `_test`. The shipped dictionary is decontaminated of the test split, so BEST-test is a genuine
  held-out set with no vocabulary leakage. Home corpus for attacut/deepcut.
- **VISTEC** — VISTEC-depa social-media corpus (Wongnai reviews). Long, messy, emoji/URL-heavy
  sentences (note the 80.1 avg tokens/sentence and 10% OOV) — the **out-of-domain stress test** for
  informal text.
- **TNHC** — Thai National Historical Corpus, classical literature. Archaic spelling and heavy
  `ๆ`/`ฯ` use; used as the **dev / anchor** set and the source of the branching-entropy table
  (`tnhc_train.jsonl` → [`tools/build_char_entropy.rb`](../tools/build_char_entropy.rb)).
- **ws1000** — a small (993-sentence) social/web eval-only set; high OOV, short sentences.
- **ORCHID** — 1980s academic-conference proceedings on the **LEXiTRON-era annotation standard**,
  which differs from the others (it glues some date/number expressions into single tokens with
  internal spaces, e.g. `"ที่ 1"`). Kept **out of the macro-average** and reported as a labelled
  out-of-domain probe, because it favors the dictionary maximal-matchers by annotation standard, not
  quality. Derived from the raw NECTEC `orchid97.txt` via
  [`tools/orchid_to_jsonl.rb`](../tools/orchid_to_jsonl.rb) (decodes `<space>`/`<minus>` escapes,
  deterministic shuffle seed=42).

### Training / auxiliary splits (not scored, but present)

| file | sentences | role |
|---|--:|---|
| `LST20_train_cleaned.jsonl` | 63,310 | dictionary + LM vocabulary (shipped model) |
| `BEST_train_cleaned.jsonl` | 111,337 | dictionary vocabulary source |
| `VISTEC_train.jsonl` | 40,000 | fair-eval single-corpus vocab source |
| `tnhc_train.jsonl` | 17,609 | branching-entropy table source |

> **Licensing:** the corpora are **not committed** to the repo (large + redistribution-restricted).
> See [`datasets/README.md`](../datasets/README.md) for where to obtain each and
> [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) for terms.

---

## Corpus file format

Every corpus is **JSON Lines**: one sentence per line, each line a **JSON array of gold token
strings** that tile the sentence exactly — **whitespace is itself a token**.

```jsonl
["ฉัน","รัก","ภาษา","ไทย"]
["รมว.","ไอซีที"," ","ยืนยัน"," ","3จี"," ","เฟส","แรก"]
```

Because tokens tile the text exactly, `"".join(tokens)` reconstructs the raw sentence, and cumulative
token length gives each word's `[start, end)` character offset — which is what every span-based metric
here relies on. The `*_cleaned` files are pre-normalized; scores are only comparable against the same
preprocessing.

---

## How scores are scored (metrics)

Two levels, both from the **research-standard** metric the AttaCut/DeepCut papers use
(`pythainlp.benchmarks.word_tokenization`). Whitespace is stripped by the reference `preprocessing()`
before scoring.

- **char-level F1** — word-**boundary** detection. Each character is labelled start-of-word or not;
  F1 over those labels. Forgiving (a one-character slip costs little).
- **word-level F1** — a predicted word is correct **iff *both* of its boundaries match** the gold
  word. Strict (the number users care about).

Both are **micro-averaged**: true/false positives/negatives are aggregated across *all* sentences,
then F1 is computed once —

```
precision = correct_words / predicted_words
recall    = correct_words / reference_words
F1        = 2·P·R / (P + R)
```

Macro-averaging (per-sentence F1 then mean, as some papers report) runs a few tenths to ~1.5 points
lower and is **not** directly comparable — everything here is micro.

**Two scorer implementations, one metric:**

| scorer | language | file | used for |
|---|---|---|---|
| pythainlp reference | Python | [`test/benchmark_accuracy.py`](../test/benchmark_accuracy.py) | **all published cross-engine tables** (apples-to-apples with the neural/dict baselines) |
| internal span scorer | Ruby | [`test/eval_segment.rb`](../test/eval_segment.rb) | fast Thapthim-only iteration; reports P/R/F1 both **incl.** and **excl.** whitespace tokens |

The Ruby scorer exists so development doesn't need the Python venv; the **published** numbers all go
through the Python reference scorer so Thapthim and the baselines are judged by one identical
implementation.

---

## Accuracy — the two-stage pipeline

Prediction generation (Ruby/Rust) is kept **separate** from scoring (Python) so every engine passes
through one scorer:

1. **Dump Thapthim's predictions** — [`test/dump_segmentation.rb`](../test/dump_segmentation.rb)
   writes one predicted token-array per gold sentence to `<dir>/<corpus>.jsonl`, under the same
   per-corpus caps as the scorer.
   ```bash
   ruby test/dump_segmentation.rb /tmp/pred_lst20
   THAPTHIM_LM=best ruby test/dump_segmentation.rb /tmp/pred_best   # gated BEST LM
   ```
2. **Score every engine** with the identical metric —
   [`test/benchmark_accuracy.py`](../test/benchmark_accuracy.py). Baselines are tokenized natively
   inside this script; Thapthim is read from the dump via `--pred`.
   ```bash
   /tmp/thai_bench/bin/python test/benchmark_accuracy.py thapthim-LST20 --pred /tmp/pred_lst20
   /tmp/thai_bench/bin/python test/benchmark_accuracy.py deepcut         # native
   ```

The scorer also prints `recon_mism` — sentences whose predicted tokens don't rejoin to the exact input
— a guard that a tokenizer isn't dropping/adding characters. Published runs have every engine
reconstructing every sentence exactly.

**Home-corpus caveat (read every table with this):** each engine has a training corpus where its score
is inflated — Thapthim-LST20→LST20, Thapthim-BEST/attacut/deepcut→BEST, nlpo3/newmm→a LEXiTRON-style
dictionary (→ORCHID). The fair read is each engine on its **home** corpus **plus** the out-of-domain
ones (tnhc/vistec/ws1000/orchid). The `·fair` tables in BENCHMARKS.md strip Thapthim to a single
corpus to isolate the *method* from the union-dictionary advantage.

---

## Speed — how throughput is measured

Pure tokenization throughput, **warm** and **single-threaded per-call** — the only basis comparable
across a Rust FFI, a Rust native binding, and two Python neural models.

**Protocol** (identical for Thapthim and baselines):

- **Corpus:** LST20 test text (raw sentences, whitespace included).
- **Warmup:** the first 100 sentences are tokenized untimed (JIT/cache warm, tries built, LM
  deserialized).
- **Timing:** best-of-N full passes over the corpus. Ruby uses `Process::CLOCK_MONOTONIC`; Python uses
  `time.perf_counter()`. **best** (min time = max char/s) is reported; mean is also printed.
- **Unit:** characters/second = `total_chars / elapsed`.

| engine | how it's timed | script |
|---|---|---|
| **thapthim** (Ruby FFI) | best-of-5, warm | [`test/benchmark_speed.rb`](../test/benchmark_speed.rb) — `ruby test/benchmark_speed.rb 5000 5` |
| thapthim (Python PyO3) | same protocol; adds a `word_segment_batch` line (GIL-released, multicore) | [`test/benchmark_speed.py`](../test/benchmark_speed.py) |
| nlpo3 / newmm / attacut / deepcut | timed inside the accuracy scorer (100-sentence warmup, `perf_counter`) | [`test/benchmark_accuracy.py`](../test/benchmark_accuracy.py) |

Published figures: nlpo3 ~3.8M, **thapthim ~2.9M**, newmm ~1.1M, attacut ~95k, deepcut ~3.5k char/s
(single-threaded per-call). The Python binding runs the same engine at ~3.2M char/s per-call;
`word_segment_batch` reaches ~11M char/s on 8 cores — a **multicore-deployment** figure, not engine
speed. The first call in a fresh process pays a one-time ~0.2 s bootstrap (LM deserialize, TCC regex
compile, trie build), amortized to nothing in any server/batch workload and excluded by the warmup.

---

## OOV recall — how generalization is measured

Word-F1 is dominated by frequent in-dictionary words and hides how an engine handles **unseen** ones.
The OOV harness ([`test/eval_oov.rb`](../test/eval_oov.rb)) is SIGHAN-style: it splits every gold word
into two buckets by dictionary membership and reports **recall** in each.

- **In-vocabulary vs OOV** is decided by **one shared reference** — Thapthim's shipped lexicon
  (`ext/thapthim/assets/master_words_vocab.txt`, **141,548 words**). A gold word is OOV **iff absent
  from that set**, so *every* engine is scored on the *identical* OOV word set (no engine is judged
  against its own dictionary).
- **Recalled** = the gold word's exact `[start, end)` character span appears in the prediction.
  Whitespace-only tokens are excluded from both buckets.
- **R<sub>iv</sub>** = recall on in-vocab words (memorization). **R<sub>oov</sub>** = recall on OOV
  words (generalization — the number the branching-entropy merge post-pass is meant to move).

```bash
ruby test/eval_oov.rb                 # Thapthim, every corpus
ruby test/eval_oov.rb tnhc lst20      # subset
```

For the **cross-model** OOV table, the same OOV reference is applied to every engine's dumped
predictions via [`test/eval_oov_compare.py`](../test/eval_oov_compare.py). OOV caps differ slightly
from the accuracy caps (lst20 5,250 · best 3,000 · vistec 3,000 · tnhc 4,403 · ws1000 993 · orchid
3,000) and are stated in BENCHMARKS.md. This is Thapthim's weakest dimension (~0.25 micro
R<sub>oov</sub>), which is why it is measured explicitly rather than folded into F1.

---

## POS tagging — how accuracy is measured

`pos_tag` is a first-order (bigram) HMM over the 16-tag LST20 tagset, evaluated by
[`test/eval_pos.rb`](../test/eval_pos.rb).

- **Gold word tokens** are fed in (so a segmentation error can't contaminate the tagging number).
- Split into **known-word** vs **OOV-word** accuracy.
- The LST20 space token (`_`→PU) is ~16% of tokens and trivially 100% correct, so it is **excluded
  from scoring** (still fed to the tagger as context).

Set: LST20 test — 5,250 sentences, 207,278 tokens, of which **174,074 are scored**. Result: 92.90%
overall (93.37% known, 70.23% OOV, OOV = 2.03% of scored tokens). Reproduce:
`ruby test/eval_pos.rb datasets/LST20_full_train datasets/LST20_full_test`.

---

## Spelling correction — how it is measured

[`test/eval_correct.rb`](../test/eval_correct.rb) auto-detects the corpus shape per file and applies
one of two metrics:

- **NO-HARM** (clean corpora — a JSONL of gold token arrays, e.g. lst20/best/tnhc): run the corrector
  on already-correct text and measure how often it **changes** a word. Altering correct text is the
  expensive error (the analogue of "don't break R<sub>iv</sub>"); target ≪ 1%. An identity corrector
  must score exactly 0.00%, which is how the harness is validated.
- **CORRECTION** (typo corpora — `{"err":[...], "cor":[...]}` pairs, e.g. VISTEC-TP-TH-2021):
  sentence-level exact-correction accuracy — the fraction of erroneous sentences whose corrected
  output matches the gold correction, after identical normalization of both sides (so normalization
  differences are never counted as correction errors).

```bash
ruby test/eval_correct.rb            # no-harm on clean corpora
ruby test/eval_correct.rb /path/vistec_tp.jsonl   # correction eval on a typo corpus
```

---

## Syllable segmentation — how it is measured

`syllable_segment` is scored by **agreement with its training target**, PyThaiNLP's SSG
(`engine="ssg"`), since its LM is trained on SSG output per gold word. Metric: per-word boundary F1 on
LST20 → **0.9941** (near-perfect reproduction). The raw SSG-on-full-text F1 of 0.81 is a
space/number tokenization-convention gap, not a quality difference.

---

## The shipped assets (in-repo resources)

Unlike the corpora, these **are** committed — they are the model, and they double as measurement
resources (the OOV reference, the entropy table). They live in `ext/thapthim/assets/`.

| asset | what it is | built by |
|---|---|---|
| `master_words_vocab.txt` | 141,548-word lexicon; **the shared OOV reference** | asset notebook (LST20 ∪ BEST ∪ PyThaiNLP) |
| word/syllable LM tables | KN-bigram Viterbi weights (`d = 0.75`) | asset notebook (LST20-trained, shipped) |
| char branching-entropy table | drives the LM-independent OOV merge | [`tools/build_char_entropy.rb`](../tools/build_char_entropy.rb) (TNHC-train) |
| `spell_words.txt`, `spell_unigrams/bigrams_aligned.txt` | spelling-corrector dictionary + LMs | `tools/build_spell_*.rb` |

See [ARCHITECTURE.md](ARCHITECTURE.md) for how these feed the grid-Viterbi engine, and
[BENCHMARKS.md](BENCHMARKS.md) for the results they produce.
