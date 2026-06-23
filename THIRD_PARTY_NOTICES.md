# Third-Party Notices

Thapthim's **source code** is released under the [MIT License](LICENSE.txt). However, it
incorporates third-party source code (Apache-2.0) and — more importantly — **dictionaries and a
statistical model derived from non-commercial corpora**. This file records those sources, their
terms, and the resulting licensing constraint on the project as a whole.

> **Status legend** — `[VERIFY]` marks a term not yet confirmed against an authoritative source.
> Items without the tag are confirmed from license documents or headers in hand.

---

## ⚠️ 0. Licensing status of the project as a whole (read first)

The shipped model assets are derived from **non-commercial** corpora, which constrains the entire
package regardless of the MIT license on the code:

- The **dictionary** includes vocabulary from **BEST**, licensed **CC BY-NC-SA 3.0** (NonCommercial
  **+ ShareAlike**).
- The **n-gram language model** is trained on **LST20**, whose NECTEC agreement restricts
  redistribution and **forbids commercial use without a paid license**.

**Consequence:** as currently built, Thapthim **cannot be used commercially** without separate
licenses from NECTEC. BEST's ShareAlike further pushes any redistribution of the dictionary toward a
CC BY-NC-SA license, which is incompatible with MIT.

**Resolution adopted (Option 2 in §4):** the **source code stays MIT**; the bundled model assets are
governed by their non-commercial corpus licenses; and the **gem as distributed is non-commercial /
research / open-source only.** This is stated in `LICENSE.txt`, `README.md`, and the gemspec
(`licenses = ["MIT", "CC-BY-NC-SA-3.0"]`). Commercial distribution would require the clean-room
rebuild (Option 3) or NECTEC licenses (Option 4).

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
- **Grammar:** Wittawat Jitkrittum — **jtcc**, https://github.com/wittawatj/jtcc — license **[VERIFY]**

Apache-2.0 code may be redistributed within an MIT-licensed project provided the Apache
attribution/headers are retained (they are). This part is **not** the licensing problem; the corpora
in §0/§2/§3 are.

---

## 2. Bundled dictionaries and statistical models (shipped in the gem)

Line-structured data files; their attribution lives here (they cannot carry inline headers).

### Word / syllable dictionaries — **CC BY-NC-SA 3.0** (via BEST)
- **Files:** `ext/thapthim/assets/master_words_vocab.txt`, `master_syllables_vocab.txt`
- **Derived from:** the PyThaiNLP word list (Apache-2.0, §1) ∪ vocabulary from the **BEST** corpus.
  The BEST-derived portion carries BEST's **CC BY-NC-SA 3.0** terms: attribution, **NonCommercial**,
  and **ShareAlike** (adaptations must be licensed alike). A compiled vocabulary extracted from BEST
  is reasonably treated as an adaptation of the corpus — **[VERIFY]** if you need a firm position,
  but treat as NC-SA by default.

### N-gram language model — **non-commercial** (via LST20)
- **Files:** `ext/thapthim/assets/joint_lm.bin`, `joint_lm_interned.bin`, `kn_*_{unigrams,bigrams}.txt`
- **Trained on:** the **LST20** corpus. Ships as derived statistics (counts), not corpus text.
- **Terms (from LST20's AGREEMENT):** non-commercial / research / open-source use is free **with
  mandatory citation**; **commercial use requires a paid license or a data contribution to NECTEC**;
  *"modification and redistribution of the dataset … are strictly prohibited unless authorized."*
  The agreement separately addresses models trained on LST20: it **requests** that such models, code,
  and APIs be shared via the AI for Thai project (contact thepchai@nectec.or.th). Shipping the trained
  model in an open-source, non-commercial project is consistent with these terms **provided LST20 is
  cited** and commercial use is excluded.

### Character branching-entropy table
- **File:** `ext/thapthim/assets/char_entropy.txt`
- **Built from:** the **TNHC** corpus, as derived statistics. License/citation **[VERIFY]** (see §3).

---

## 3. Evaluation / training corpora (not committed to this repository)

Used for evaluation and to build the assets above. **Not committed** (gitignored — see
`datasets/README.md`) and **not** in the gem.

### BEST — *Benchmark for Enhancing the Standard of Thai language processing* (NECTEC)
- **License:** **CC BY-NC-SA 3.0 Unported** (https://creativecommons.org/licenses/by-nc-sa/3.0/),
  per the BEST 2010 / Inter-BEST 2009 word-segmentation guideline. (The license statement appears on
  the guideline document; treat the corpus and its derived word lists as the same NC-SA terms unless
  NECTEC states otherwise — **[VERIFY]** only if a firmer position is needed.)
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

### VISTEC — VISTEC-depa Thailand AI Research Institute
- **Used for:** evaluation only — **not** compiled into the gem, not redistributed. Its license
  therefore does **not** affect the published package under the non-commercial Option 2.
- **License / citation:** unverified (license documents not on hand). Only relevant if pursuing the
  commercial clean-room path (Option 3), where VISTEC's terms would need confirming.

### TNHC — Thai literary corpus (held-out dev/anchor set; source of `char_entropy.txt`)
- **Used for:** building `char_entropy.txt` (shipped, derived statistics) and as a dev set.
- **License / citation:** unverified (license documents not on hand). Treated as a
  non-commercial / research corpus, consistent with the gem's overall non-commercial status; the
  shipped artifact is derived entropy values, not corpus text, so residual exposure is low. Re-derive
  `char_entropy.txt` from a known-licensed corpus if a firmer position is ever required.

---

## 4. Options to resolve the MIT-vs-non-commercial conflict (decide before publishing)

1. **Relicense the gem as non-commercial** (e.g. CC BY-NC-SA 4.0 for the package, or MIT code +
   an NC notice on the bundled assets). Honest and simplest, but unusual for a gem and blocks
   commercial adoption.
2. **Split licenses explicitly:** keep the **code MIT**, mark the **model/dictionary assets** as
   **CC BY-NC-SA 3.0 / LST20-NC**, and state plainly that *the gem as distributed is
   non-commercial*. Document in README + gemspec metadata.
3. **Clean-room the assets for a true MIT/commercial gem:** rebuild the dictionary from only
   permissive sources (e.g. the PyThaiNLP Apache-2.0 word list) and retrain the LM on a
   permissively-licensed corpus. Removes the NC encumbrance but costs accuracy and depends on a
   permissive Thai corpus existing (most — BEST, LST20, ORCHID — are NECTEC NC). **[VERIFY]** VISTEC's
   license as a candidate.
4. **Obtain NECTEC licenses** (purchase / data contribution for LST20; clarify BEST) if commercial
   distribution is the goal.

## Maintainer checklist before publishing

- [x] **Decide §4** — adopted **Option 2** (code MIT; gem distributed non-commercial).
- [x] Correct the gem's stated license — `LICENSE.txt`, `README.md`, and gemspec now state the
      non-commercial constraint.
- [x] Add the **mandatory LST20 citation** (in README + §3).
- [x] VISTEC / TNHC: moot for Option 2 (VISTEC eval-only; TNHC yields only derived statistics in an
      already-non-commercial gem). Revisit only if pursuing Option 3.
- [ ] jtcc license (`[VERIFY]`, §1) — minor; confirm if you want the attribution airtight.
- [x] Keep `datasets/` out of the gem and the repo (done).
