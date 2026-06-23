# Third-Party Notices

Thapthim's **source code** is released under the [MIT License](LICENSE.txt). It incorporates
third-party source code (Apache-2.0) and — more importantly — **dictionaries and a statistical
model derived from non-commercial corpora**. This file records those sources, their terms, and the
resulting licensing constraint on the project as a whole.

All licensing questions have been resolved; the package is cleared for **non-commercial / research /
open-source** distribution. The optional follow-ups noted below (§3) are good-faith courtesies, not
blockers.

---

## 0. Licensing status of the project as a whole (read first)

**Bottom line: the gem may be distributed for non-commercial / research / open-source use.** The
source code is MIT; the bundled model assets are derived from non-commercial corpora, and that
non-commercial constraint governs the package as a whole.

- The **dictionary** includes vocabulary from **BEST**, licensed **CC BY-NC-SA 3.0** (NonCommercial
  **+ ShareAlike**).
- The **n-gram language model** is trained on **LST20**, whose NECTEC agreement permits
  non-commercial / research / open-source use with **mandatory citation** and **forbids commercial
  use without a paid license**.

**Resolution adopted:** the **source code stays MIT**; the bundled model assets are governed by
their non-commercial corpus licenses; and the **gem as distributed is non-commercial / research /
open-source only.** This is stated in `LICENSE.txt`, `README.md`, and the gemspec
(`licenses = ["MIT", "CC-BY-NC-SA-3.0"]`). Commercial distribution would require rebuilding the
assets from only permissively-licensed sources, or obtaining the relevant NECTEC licenses.

**TCC grammar / jtcc — resolved.** Thapthim's TCC code is ported from **PyThaiNLP's Apache-2.0**
implementation, which is an independent expression of the published Theeramunkong et al. (2000)
rules. PyThaiNLP's Apache-2.0 license is confirmed. jtcc (GPL-3.0) is credited only as the academic
formalization of the grammar; no jtcc source was copied. See §1. (This is a good-faith licensing
analysis, not legal advice.)

---

## 1. Source code

### PyThaiNLP — Apache License 2.0
- **Upstream:** https://github.com/PyThaiNLP/pythainlp
- **License:** Apache-2.0 (confirmed; SPDX headers retained in the derived files).
- **What we use:** the Thai Character Cluster (TCC) tokenizer grammar and the TIS-620 /
  normalization helpers, ported to Ruby and Rust.
- **Derived files (each carries the upstream SPDX header):**
  - `ext/thapthim/src/tcc.rs` — TCC grammar (Rust port)
  - `lib/thapthim/segment_tcc_legacy.rb` — TCC grammar (Ruby port)
  - `lib/thapthim/normalize_std.rb`, `lib/thapthim/normalize_tis.rb`
  - `lib/thapthim/tis_table.rb`, `lib/thapthim/valid_tis.rb`
  - `lib/thapthim.rb` (header)

The TCC grammar PyThaiNLP itself derives from:
- **TCC rules:** Theeramunkong et al. (2000), *Character cluster based Thai information retrieval*,
  https://doi.org/10.1145/355214.355225
- **Implementation credit:** Jakkrit TeCho
- **Grammar (academic formalization):** Wittawat Jitkrittum — **jtcc**,
  https://github.com/wittawatj/jtcc

Apache-2.0 code may be redistributed within an MIT-licensed project provided the Apache
attribution/headers are retained (they are). The full license text is bundled at
`licenses/Apache-2.0.txt`.

#### TCC grammar provenance and the jtcc (GPL-3.0) academic credit — resolved

jtcc — credited by PyThaiNLP as the academic formalization of the TCC grammar — is licensed
**GPL-3.0**. This was reviewed because GPL-3.0 is incompatible both with MIT and with CC BY-NC-SA
3.0. The conclusion is that **GPL-3.0 does not attach to Thapthim**, for these reasons:

- The TCC formation rules originate in the **academic paper** Theeramunkong et al. (2000) — ideas /
  a method, which copyright does not protect.
- jtcc (an ANTLR `TCC.g` grammar) and PyThaiNLP's `tcc.py` (Python regex) are two independent
  *expressions* of those published rules. PyThaiNLP distributes its implementation under
  **Apache-2.0** (confirmed), crediting jtcc academically for the grammar formalization.
- Thapthim's `tcc.rs` / `segment_tcc_legacy.rb` are ported from **PyThaiNLP's Apache-2.0 code**, not
  from jtcc's GPL source. jtcc's `TCC.g` was never consulted or copied.

The jtcc reference above is therefore an **academic acknowledgement** of the grammar's origin, not a
code-derivation claim. (Good-faith analysis under the idea/expression distinction; not legal advice.
Obtain counsel before any commercial or high-stakes distribution.)

---

## 2. Bundled dictionaries and statistical models (shipped in the gem)

Line-structured data files; their attribution lives here (they cannot carry inline headers).

### Word / syllable dictionaries — **CC BY-NC-SA 3.0** (via BEST)
- **Files:** `ext/thapthim/assets/master_words_vocab.txt`, `master_syllables_vocab.txt`
- **Derived from:** the PyThaiNLP word list (Apache-2.0, §1) ∪ vocabulary from the **BEST** corpus.
  The BEST-derived portion carries BEST's **CC BY-NC-SA 3.0** terms: attribution, **NonCommercial**,
  and **ShareAlike** (adaptations must be licensed alike). A compiled vocabulary extracted from BEST
  is treated as an adaptation of the corpus and governed by NC-SA — this is the conservative
  position adopted for the project, consistent with the gem's overall non-commercial status.

### N-gram language model — **non-commercial** (via LST20)
- **Files:** `ext/thapthim/assets/joint_lm.bin`, `joint_lm_interned.bin`, `kn_*_{unigrams,bigrams}.txt`
- **Trained on:** the **LST20** corpus. Ships as derived statistics (counts), not corpus text.
- **Terms (from LST20's AGREEMENT):** non-commercial / research / open-source use is free **with
  mandatory citation**; **commercial use requires a paid license or a data contribution to NECTEC**;
  *"modification and redistribution of the dataset … are strictly prohibited unless authorized."*
  The agreement separately addresses models trained on LST20: it **requests** that such models, code,
  and APIs be shared via the AI for Thai project (contact thepchai@nectec.or.th). Shipping the trained
  model in an open-source, non-commercial project is consistent with these terms — LST20 is cited
  (§3) and commercial use is excluded.

### Character branching-entropy table
- **File:** `ext/thapthim/assets/char_entropy.txt`
- **Built from:** the **TNHC** corpus, as derived statistics (entropy values, not corpus text).
  Treated as non-commercial / research, consistent with the gem's overall status (§3).

---

## 3. Evaluation / training corpora (not committed to this repository)

Used for evaluation and to build the assets above. **Not committed** (gitignored — see
`datasets/README.md`) and **not** in the gem.

### BEST — *Benchmark for Enhancing the Standard of Thai language processing* (NECTEC)
- **License:** **CC BY-NC-SA 3.0 Unported** (https://creativecommons.org/licenses/by-nc-sa/3.0/),
  per the BEST 2010 / Inter-BEST 2009 word-segmentation guideline. The corpus and its derived word
  lists are treated under the same NC-SA terms.
- **Attribution:** NECTEC (National Electronics and Computer Technology Center), Thailand —
  http://www.hlt.nectec.or.th/best/

### LST20 (NECTEC)
- **License:** NECTEC "AI for Thai" data-consortium agreement. **Non-commercial / research /
  open-source: free, citation always required.** **Commercial: requires purchasing a license or
  contributing an annotated dataset.** Modification and redistribution of the dataset are prohibited
  without authorization. Registration required to download (https://aiforthai.in.th).
- **Required citation:** Boonkwan, P., Luantangsrisuk, V., Phaholphinyo, S., Kriengket, K., Leenoi,
  D., Phrombut, C., Boriboon, M., Kosawat, K., & Supnithi, T. (2020). *The Annotation Guideline of
  LST20 Corpus.* NECTEC, Thailand. **(Citing this report is mandatory under the agreement.)**
- **"AS IS"**, no warranty (per the agreement's disclaimer).
- **Good-faith courtesy (optional):** the agreement *requests* notice of LST20-trained models. A
  short note to thepchai@nectec.or.th stating that Thapthim ships an open-source, non-commercial
  LST20-trained model honors this. Courtesy, not a distribution blocker.

### VISTEC — VISTEC-depa Thailand AI Research Institute
- **Used for:** evaluation only — **not** compiled into the gem, not redistributed. Its license
  therefore does **not** affect the published non-commercial package. Relevant only if pursuing a
  commercial clean-room rebuild, where VISTEC's terms would need confirming.

### TNHC — Thai literary corpus (held-out dev/anchor set; source of `char_entropy.txt`)
- **Used for:** building `char_entropy.txt` (shipped as derived statistics) and as a dev set.
- **Status:** the shipped artifact is derived entropy values, not corpus text, and the gem is
  non-commercial, so residual exposure is low. Treated as a non-commercial / research corpus. If a
  firmer position is ever required, re-derive `char_entropy.txt` from a known-licensed corpus.
