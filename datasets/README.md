# Datasets

The evaluation/training corpora used by Thapthim are **not committed to this repository**. They
are large, and several are distributed under licenses that restrict redistribution (see
[../THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md)). This directory is gitignored except for
this README — obtain the corpora yourself and drop the files here to run evaluation or rebuild the
model assets.

## Expected files

Place these exact filenames in this directory:

| File | Source corpus | Consumed by |
|---|---|---|
| `LST20_train_cleaned.jsonl` | LST20 (NECTEC) | dictionary + LM vocabulary source (asset build) |
| `LST20_test_cleaned.jsonl` | LST20 (NECTEC) | `test/eval_segment.rb lst20`, `test/benchmark_speed.rb`, `test/benchmark_baselines.py` |
| `BEST_train_cleaned.jsonl` | BEST (NECTEC) | dictionary vocabulary source; `test/eval_segment.rb best_train` |
| `BEST_test_cleaned.jsonl` | BEST (NECTEC) | `test/eval_segment.rb best` (held-out eval set) |
| `VISTEC_test.jsonl`, `VISTEC_train.jsonl` | VISTEC-depa | `test/eval_segment.rb vistec` |
| `tnhc_test.jsonl` | TNHC | `test/eval_segment.rb tnhc` |
| `tnhc_train.jsonl` | TNHC | `tools/build_char_entropy.rb` (builds the branching-entropy table) |

`BEST_{train,test}_cleaned.jsonl` are a deduplicated, shuffled 80:20 split (seed=42) of the
BEST corpus — string-disjoint, so no sentence appears in both. The shipped dictionary is built
from `BEST_train_cleaned.jsonl` only (plus LST20-train and PyThaiNLP), so `BEST_test` is a
genuinely held-out evaluation set with no vocabulary leakage.

## File format

Each file is JSON Lines: **one sentence per line**, where each line is an **array of gold token
strings** that tile the sentence exactly (whitespace is a token). Example:

```jsonl
["ฉัน", "รัก", "ภาษา", "ไทย"]
["รมว.", "ไอซีที", " ", "ยืนยัน"]
```

The `*_cleaned` files are **preprocessed** (normalization/cleaning applied before evaluation). The
exact cleaning affects reported scores, so numbers are only comparable against the same
preprocessing — see the project history/notebook for the cleaning steps.

## Where to obtain (verify the license for your use)

> URLs and license terms change; confirm each against the upstream source before downloading or
> redistributing. The trained model shipped in the gem is derived from **LST20** — check its terms
> first.

- **LST20** — NECTEC. Distributed via AI for Thai / ELRA under a NECTEC license agreement (may be
  non-commercial). https://aiforthai.in.th/
- **BEST** — NECTEC, *Benchmark for Enhancing the Standard of Thai language processing*.
  https://aiforthai.in.th/
- **VISTEC** — VISTEC-depa Thailand text-processing corpus (commonly on GitHub under a Creative
  Commons license). Search "VISTEC-depa Thai word segmentation".
- **TNHC** — Thai literary corpus used here as the held-out dev/anchor set.

Once the files are in place, `ruby test/eval_segment.rb` will pick them up automatically.
