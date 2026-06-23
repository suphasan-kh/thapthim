# Third-Party Notices

Thapthim itself is released under the [MIT License](LICENSE.txt). However, it incorporates
source code, dictionaries, statistical models, and (in the repository, not the published gem)
evaluation corpora that originate from third parties under their own licenses. This file records
those sources and their terms.

> **Status legend** — `[VERIFY]` marks a license/term that has **not** been confirmed against the
> upstream source and must be checked before relying on it (especially before publishing or
> redistributing). Items without the tag are confirmed from headers shipped in this repository.

---

## 1. Source code

### PyThaiNLP — Apache License 2.0
- **Upstream:** https://github.com/PyThaiNLP/pythainlp
- **License:** Apache-2.0 (confirmed; SPDX headers retained in the derived files).
- **What we use:** the Thai Character Cluster (TCC) tokenizer grammar and the TIS-620 /
  normalization helpers, ported to Ruby and Rust.
- **Derived files in this project (each carries the upstream SPDX header):**
  - `ext/thapthim/src/tcc.rs` — TCC grammar (Rust port)
  - `lib/thapthim/segment_tcc_legacy.rb` — TCC grammar (Ruby port)
  - `lib/thapthim/normalize_std.rb`, `lib/thapthim/normalize_tis.rb`
  - `lib/thapthim/tis_table.rb`, `lib/thapthim/valid_tis.rb`
  - `lib/thapthim.rb` (header)

The TCC grammar PyThaiNLP itself derives from:
- **TCC rules:** Theeramunkong et al. (2000), *Character cluster based Thai information retrieval*,
  https://doi.org/10.1145/355214.355225
- **Implementation credit:** Jakkrit TeCho
- **Grammar:** Wittawat Jitkrittum — **jtcc**, https://github.com/wittawatj/jtcc — license **[VERIFY]**

> **Note on overall licensing:** the gemspec and `LICENSE.txt` declare **MIT**, but the files above
> are **Apache-2.0**. Apache-2.0 code may be redistributed within an MIT-licensed project provided
> the Apache attribution/headers are retained (they are). Consider stating this mixed licensing in
> the README before publishing. **[VERIFY]** with your own reading of both licenses.

---

## 2. Bundled dictionaries and statistical models (shipped in the gem)

These are line-structured data files and therefore cannot carry inline SPDX headers; their
attribution lives here.

### Word / syllable dictionaries
- **Files:** `ext/thapthim/assets/master_words_vocab.txt`, `master_syllables_vocab.txt`
- **Derived from:** the PyThaiNLP word list (Apache-2.0, above) ∪ vocabulary observed in the
  **BEST** corpus (see §3). **[VERIFY]** the redistribution terms of the BEST-derived entries.

### N-gram language model
- **Files:** `ext/thapthim/assets/joint_lm.bin`, `joint_lm_interned.bin`, `kn_*_{unigrams,bigrams}.txt`
- **Trained on:** the **LST20** corpus (see §3). The model ships as derived statistics (counts),
  not the corpus text. **[VERIFY]** whether LST20's license permits redistributing a model
  trained on it — LST20 is distributed under a NECTEC license that may restrict this.

### Character branching-entropy table
- **File:** `ext/thapthim/assets/char_entropy.txt`
- **Built from:** the **TNHC** corpus (see §3), as derived entropy statistics. **[VERIFY]**
  redistribution terms.

---

## 3. Evaluation / training corpora (not committed to this repository)

Used for evaluation and to build the models above. These are **not committed** (gitignored — see
`datasets/README.md` for how to obtain them) and **not** packaged in the gem. Each corpus is the
property of its creators under its own license and citation requirements.

### BEST — `datasets/BEST_train_cleaned.json`
- **Source:** NECTEC, *BEST: Benchmark for Enhancing the Standard of Thai language processing*.
- **License / citation:** **[VERIFY]** (NECTEC terms).

### LST20 — `datasets/LST20_test_cleaned.json`
- **Source:** NECTEC, LST20 corpus. Citation: Boonkwan et al. (2020), *The Annotation Guideline of
  LST20 Corpus*. **[VERIFY]**
- **License / citation:** **[VERIFY]** — LST20 is distributed under a NECTEC license agreement that
  may impose **non-commercial** and **redistribution** restrictions. This is the most important
  item to confirm, because the shipped n-gram model (§2) is trained on it.

### VISTEC — `datasets/VISTEC_test.json`, `datasets/VISTEC_train.json`
- **Source:** VISTEC-depa Thailand AI Research Institute, Thai text-processing corpus.
- **License / citation:** **[VERIFY]** (believed to be a Creative Commons license — confirm which).

### TNHC — `datasets/tnhc_test.json`, `datasets/tnhc_train.json`
- **Source:** TNHC Thai literary corpus (used here as the held-out dev/anchor set and to build the
  branching-entropy table).
- **Full name / license / citation:** **[VERIFY]**.

---

## Maintainer checklist before publishing

- [ ] Resolve every `[VERIFY]` above against the upstream license text.
- [ ] Confirm LST20's terms permit shipping the trained n-gram model in a (possibly commercial) gem.
- [ ] State the MIT-project / Apache-2.0-components mixed licensing in the README.
- [ ] Keep `datasets/` out of the published gem (already excluded in the gemspec).
