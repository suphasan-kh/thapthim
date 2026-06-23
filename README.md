# Thapthim (ทับทิม)

Thai word and syllable segmentation for Ruby, backed by a native Rust extension.

Thai is written without spaces between words, so almost any Thai NLP task starts with segmentation.
Thapthim splits Thai text into **words** and **orthographic syllables** using a dictionary lattice
aligned to Thai Character Cluster (TCC) boundaries, scored with a Kneser-Ney bigram language model
via Viterbi decoding, with branching-entropy back-off for out-of-vocabulary spans.

```ruby
Thapthim.segment("ฉันกินข้าว")    # => ["ฉัน", "กิน", "ข้าว"]
Thapthim.syllables("ฉันกินข้าว")  # => ["ฉัน", "กิน", "ข้าว"]
```

## Installation

> The gem is not yet published to RubyGems. Install from source for now.

```bash
git clone https://github.com/suphasan-kh/thapthim.git
cd thapthim
bundle install
bundle exec rake compile   # builds the Rust extension
```

Building the native extension requires a **Rust toolchain** (`rustc` / `cargo`, via
[rustup](https://rustup.rs)) in addition to Ruby ≥ 3.2.

## Usage

```ruby
require "thapthim"

# Word segmentation — the main entry point.
Thapthim.segment("ฉันกินข้าว")
# => ["ฉัน", "กิน", "ข้าว"]

# Syllable segmentation — boundaries are a superset of the word boundaries.
Thapthim.syllables("ฉันกินข้าว")
# => ["ฉัน", "กิน", "ข้าว"]

# Optional text normalization before segmenting (collapses spaces, reorders vowels,
# strips zero-width characters, removes repeated marks, etc.).
Thapthim.segment("  ฉัน   กิน  ", normalize: true)

# Normalize on its own.
Thapthim.std_normalize("  ฉัน   กิน  ")
# => "ฉัน กิน"

# Thai Character Cluster (TCC) segmentation — the lowest-level inseparable units.
Thapthim.tcc_segment("ฉันกินข้าว")
# => ["ฉั", "น", "กิ", "น", "ข้า", "ว"]
```

All methods accept any input and harden it on the way in: non–UTF-8 encodings (e.g. TIS-620) are
transcoded, invalid bytes are scrubbed, and embedded NUL bytes are stripped, so segmentation never
crashes on malformed input.

## How it works

1. **TCC grid** — the input is divided into Thai Character Clusters, the smallest units that can
   never be split mid-word. All candidate word boundaries are constrained to this grid.
2. **Dictionary lattice** — dictionary words are matched over the grid with a
   [daachorse](https://github.com/daac-tools/daachorse) Aho-Corasick automaton.
3. **Viterbi decoding** — the best path through the lattice is chosen with a Kneser-Ney smoothed
   bigram language model.
4. **OOV back-off** — spans with no dictionary coverage fall back to a branching-entropy merge over
   TCC units, so unknown words still segment sensibly.

The dictionary, language model, and entropy table are compiled into the gem as data assets.

## Development

After checking out the repo, run `bundle install`, then `bundle exec rake compile` to build the
extension and `bundle exec rake spec` (or `rake test`) to run the tests. `rake` with no arguments
compiles and tests.

Training/evaluation corpora are **not** committed (size + corpus licenses) — see
[datasets/README.md](datasets/README.md) for the expected files and how to obtain them. The model
assets shipped in the gem are built from these; the build tooling lives in `tools/`.

## License

**Source code:** [MIT License](https://opensource.org/licenses/MIT) (see [LICENSE.txt](LICENSE.txt)).

**The gem as a whole is for non-commercial / research / open-source use only.** This is *not* a
choice — it follows from the bundled model assets, which are derived from non-commercial corpora:

- The dictionary includes vocabulary from the **BEST** corpus — **CC BY-NC-SA 3.0** (NonCommercial,
  ShareAlike).
- The n-gram language model is trained on the **LST20** corpus — NECTEC's agreement permits
  non-commercial/research/open-source use **only**, and **requires citing** the LST20 report
  (Boonkwan et al., 2020, *The Annotation Guideline of LST20 Corpus*). Commercial use requires a
  separate license from NECTEC.

So while the code is MIT, you **may not use the gem commercially** without resolving the corpus
licenses yourself. It also bundles PyThaiNLP's TCC/normalization components (Apache-2.0). Full
attribution and per-source terms are in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Contributing

Bug reports and pull requests are welcome on GitHub at
https://github.com/suphasan-kh/thapthim. This project is intended to be a safe, welcoming space for
collaboration, and contributors are expected to adhere to the
[code of conduct](https://github.com/suphasan-kh/thapthim/blob/main/CODE_OF_CONDUCT.md).

## Code of Conduct

Everyone interacting in the Thapthim project's codebases, issue trackers, chat rooms and mailing
lists is expected to follow the
[code of conduct](https://github.com/suphasan-kh/thapthim/blob/main/CODE_OF_CONDUCT.md).
