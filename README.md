# Thapthim (ทับทิม)

Thai word and syllable segmentation for Ruby (and Python), backed by a native Rust extension.

Thai is written without spaces between words, so almost any Thai NLP task starts with segmentation.
Thapthim splits Thai text into **words** and **orthographic syllables** using a dictionary lattice
aligned to Thai Character Cluster (TCC) boundaries, scored with a Kneser-Ney bigram language model
via Viterbi decoding, with a branching-entropy back-off for out-of-vocabulary spans.

```ruby
Thapthim.word_segment("ฉันกินข้าว")      # => ["ฉัน", "กิน", "ข้าว"]
Thapthim.syllable_segment("ฉันกินข้าว")  # => ["ฉัน", "กิน", "ข้าว"]
```

## Installation (Ruby)

> Not yet published to RubyGems — install from source. Requires a **Rust toolchain**
> (`rustc`/`cargo`, via [rustup](https://rustup.rs)) and Ruby ≥ 3.2.

```bash
git clone https://github.com/suphasan-kh/thapthim.git
cd thapthim
bundle install
bundle exec rake install   # builds the Rust extension and installs the gem
```

For in-repo development use `bundle exec rake compile` (builds in place), then run with `bundle
exec` or `ruby -Ilib`.

## Usage (Ruby)

```ruby
require "thapthim"

Thapthim.word_segment("ฉันกินข้าว")        # => ["ฉัน", "กิน", "ข้าว"]
Thapthim.syllable_segment("ฉันกินข้าว")    # boundaries are a superset of the word boundaries

# Optional normalization before segmenting (collapses spaces, reorders vowels, strips
# zero-width chars, removes repeated marks).
Thapthim.word_segment("  ฉัน   กิน  ", normalize: true)
Thapthim.std_normalize("  ฉัน   กิน  ")    # => "ฉัน กิน"

# Thai Character Clusters — the lowest-level inseparable units.
Thapthim.tcc_segment("ฉันกินข้าว")         # => ["ฉั", "น", "กิ", "น", "ข้า", "ว"]
```

Every entry point hardens its input — non–UTF-8 (e.g. TIS-620) is transcoded, invalid bytes and
embedded NULs are scrubbed — so segmentation never crashes on malformed text.

## Python

The same Rust engine is exposed to Python via [PyO3](https://pyo3.rs)/[maturin](https://www.maturin.rs);
the API mirrors Ruby (identical engine, assets, results). Needs a Rust toolchain and Python ≥ 3.8.

```bash
pip install .                       # build + install from a clone (simplest)
# editable workflow (needs a venv):
pip install 'maturin>=1.7,<2.0' && maturin develop --release
```

```python
import thapthim
thapthim.word_segment("ฉันกินข้าว")               # ['ฉัน', 'กิน', 'ข้าว']
thapthim.syllable_segment("ฉันกินข้าว")
thapthim.word_segment("  ฉัน  ", normalize=True)
thapthim.tcc_segment("ฉันกินข้าว")
thapthim.word_segment_offsets("ฉันกิน")           # [(0, 9), (9, 9)]  (start_byte, length)
thapthim.word_segment_batch(["ฉันกิน", "ข้าว"])   # bulk: releases the GIL, fans across cores
```

On **Google Colab** (no Rust by default) install the toolchain first, then build from git:

```python
!curl https://sh.rustup.rs -sSf | sh -s -- -y
import os; os.environ["PATH"] += ":" + os.path.expanduser("~/.cargo/bin")
!pip install "git+https://github.com/suphasan-kh/thapthim.git"   # ~1–3 min to compile
```

## How it works

1. **TCC grid** — text is split into Thai Character Clusters, the smallest units that can't be split
   mid-word; all candidate boundaries snap to this grid.
2. **Dictionary lattice** — dictionary words are matched over the grid with a
   [daachorse](https://github.com/daac-tools/daachorse) Aho-Corasick automaton.
3. **Viterbi decoding** — the best path is chosen with a Kneser-Ney bigram language model.
4. **OOV back-off** — spans with no dictionary coverage fall back to a branching-entropy merge over
   TCC units, so unknown words still segment sensibly.

The dictionary, language model, and entropy table ship as compiled-in data assets. Full pipeline:
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Accuracy/speed: [docs/BENCHMARKS.md](docs/BENCHMARKS.md).

## Benchmarks

On the research-standard `pythainlp.benchmarks` metric the shipped LST20 model is competitive with the
neural baselines — topping them on 4 of 5 Thai corpora — at dictionary-class speed (~30× faster than
attacut, ~820× faster than deepcut, single-threaded). Read that with care: part of the margin is a
home-corpus advantage (the neural baselines are BEST-trained, run out-of-domain on the other four), the
dictionary tool nlpO3 is faster still, and out-of-vocabulary recall is a weak spot where the neural
models lead. The architecture is a well-known one (dictionary + n-gram Viterbi); the contribution is
the controlled finding and the compact artifact. Full tables, methodology, and caveats:
[docs/BENCHMARKS.md](docs/BENCHMARKS.md).

## Development

`bundle install`, then `bundle exec rake compile` to build and `rake test` to run the suite (`rake`
alone does both). Training/eval corpora are **not** committed (size + licenses) — see
[datasets/README.md](datasets/README.md); model assets are built from them via `tools/`.

## License

**Source code:** [MIT](https://opensource.org/licenses/MIT) (see [LICENSE.txt](LICENSE.txt)).

**The gem as a whole is non-commercial / research / open-source use only** — a consequence of the
bundled model assets, not a choice:

- The dictionary draws vocabulary from **BEST** (CC BY-NC-SA 3.0) and **LST20** (NECTEC,
  non-commercial).
- The n-gram LM is trained on **LST20**, whose agreement permits non-commercial/research/open-source
  use only and **requires citing** the LST20 report (Boonkwan et al., 2020). Commercial use needs a
  separate NECTEC license.

It also bundles PyThaiNLP's TCC/normalization components (Apache-2.0). Full per-source terms:
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Contributing

Bug reports and PRs welcome at https://github.com/suphasan-kh/thapthim. Contributors follow the
[code of conduct](https://github.com/suphasan-kh/thapthim/blob/main/CODE_OF_CONDUCT.md).
