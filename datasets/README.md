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
| `LST20_test_cleaned.json` | LST20 (NECTEC) | `test/eval_segment.rb lst20`, `test/benchmark_speed.rb`, `test/benchmark_baselines.py` |
| `BEST_train_cleaned.json` | BEST (NECTEC) | `test/eval_segment.rb best` |
| `VISTEC_test.json`, `VISTEC_train.json` | VISTEC-depa | `test/eval_segment.rb vistec` |
| `tnhc_test.json` | TNHC | `test/eval_segment.rb tnhc` |
| `tnhc_train.json` | TNHC | `tools/build_char_entropy.rb` (builds the branching-entropy table) |

## File format

Each file is JSON: an **array of sentences**, where each sentence is an **array of gold token
strings** that tile the sentence exactly (whitespace is a token). Example:

```json
[
  ["ฉัน", "รัก", "ภาษา", "ไทย"],
  ["รมว.", "ไอซีที", " ", "ยืนยัน"]
]
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
