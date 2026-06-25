# Thapthim (ทับทิม)

Thai word and syllable segmentation for Ruby, backed by a native Rust extension.

Thai is written without spaces between words, so almost any Thai NLP task starts with segmentation.
Thapthim splits Thai text into **words** and **orthographic syllables** using a dictionary lattice
aligned to Thai Character Cluster (TCC) boundaries, scored with a Kneser-Ney bigram language model
via Viterbi decoding, with branching-entropy back-off for out-of-vocabulary spans.

```ruby
Thapthim.word_segment("ฉันกินข้าว")    # => ["ฉัน", "กิน", "ข้าว"]
Thapthim.syllable_segment("ฉันกินข้าว")  # => ["ฉัน", "กิน", "ข้าว"]
```

## Installation

> The gem is not yet published to RubyGems. Install from source for now.

```bash
git clone https://github.com/suphasan-kh/thapthim.git
cd thapthim
bundle install
bundle exec rake install   # builds the Rust extension and installs the gem
```

`rake install` builds the native extension and installs the gem into your local gem repo, so
`require "thapthim"` then works from anywhere. For in-repo development, run `bundle exec rake
compile` instead — it builds the extension in place; then run your code with `bundle exec`
(or `ruby -Ilib`) so the freshly built `lib/` is on the load path.

Building the native extension requires a **Rust toolchain** (`rustc` / `cargo`, via
[rustup](https://rustup.rs)) in addition to Ruby ≥ 3.2.

## Usage

```ruby
require "thapthim"

# Word segmentation — the main entry point.
Thapthim.word_segment("ฉันกินข้าว")
# => ["ฉัน", "กิน", "ข้าว"]

# Syllable segmentation — boundaries are a superset of the word boundaries.
Thapthim.syllable_segment("ฉันกินข้าว")
# => ["ฉัน", "กิน", "ข้าว"]

# Optional text normalization before segmenting (collapses spaces, reorders vowels,
# strips zero-width characters, removes repeated marks, etc.).
Thapthim.word_segment("  ฉัน   กิน  ", normalize: true)

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

## Python

The same Rust engine is exposed to Python through [PyO3](https://pyo3.rs), built with
[maturin](https://www.maturin.rs). The API mirrors the Ruby one (identical engine, assets, and
results); only the surface syntax differs.

### Installation

> Not yet published to PyPI. Build from source for now — this needs a **Rust toolchain**
> (`rustc` / `cargo`, via [rustup](https://rustup.rs)) and **Python ≥ 3.8**.

```bash
git clone https://github.com/suphasan-kh/thapthim.git
cd thapthim
pip install .                # builds the Rust engine and installs thapthim (simplest)
```

`pip install .` is self-contained: it builds the extension through the maturin backend (pulled in
automatically — no separate install) and installs `thapthim` into the current environment. Use a
virtualenv first if you want to keep it isolated
(`python3 -m venv .venv && source .venv/bin/activate`).

For an **editable / iterative** workflow, install maturin and use `develop` instead — this path
*does* require an active virtualenv:

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install 'maturin>=1.7,<2.0'   # version pinned to match pyproject.toml
maturin develop --release         # compile + install into the venv; re-run after Rust changes
# or
maturin build --release           # just produce a wheel under target/wheels/
```

`maturin build` writes a `.whl` you can `pip install` elsewhere — but a native wheel is specific to
the Python version, OS, and CPU architecture it was built on, so it only installs on a matching
interpreter.

### Google Colab / Jupyter

Colab and most notebook images ship Python but **no Rust toolchain**, so install one first, then
build straight from the repo. Run these in two cells (or one):

```python
# 1. Install a Rust toolchain — Colab has none by default (~30s).
!curl https://sh.rustup.rs -sSf | sh -s -- -y
import os
os.environ["PATH"] += ":" + os.path.expanduser("~/.cargo/bin")
```

```python
# 2. Build + install thapthim from source (~1–3 min to compile the engine).
!pip install "git+https://github.com/suphasan-kh/thapthim.git"
```

```python
import thapthim
thapthim.word_segment("ฉันกินข้าว")     # ['ฉัน', 'กิน', 'ข้าว']
thapthim.syllable_segment("ฉันกินข้าว")   # ['ฉัน', 'กิน', 'ข้าว']
```

No runtime restart is needed. On a **local Jupyter** where Rust is already installed, skip step 1 —
just run the `!pip install "git+…"` cell (or `!pip install .` from a local clone).

### Usage

```python
import thapthim

# Word and syllable segmentation — the main entry points.
thapthim.word_segment("ฉันกินข้าว")              # ['ฉัน', 'กิน', 'ข้าว']
thapthim.syllable_segment("ฉันกินข้าว")            # ['ฉัน', 'กิน', 'ข้าว']

# Optional normalization before segmenting (parity with the Ruby `normalize:` option).
thapthim.word_segment("  ฉัน   กิน  ", normalize=True)
thapthim.std_normalize("  ฉัน   กิน  ")      # 'ฉัน กิน'

# Thai Character Cluster (TCC) segmentation — the smallest inseparable units.
thapthim.tcc_segment("ฉันกินข้าว")          # ['ฉั', 'น', 'กิ', 'น', 'ข้า', 'ว']

# Byte offsets instead of substrings — (start_byte, length_byte) per word.
thapthim.word_segment_offsets("ฉันกิน")          # [(0, 9), (9, 9)]

# Batch path: segments a list in one boundary crossing, releasing the GIL and
# fanning across cores — for bulk throughput, not single-call latency.
thapthim.word_segment_batch(["ฉันกิน", "ข้าว"])  # [['ฉัน', 'กิน'], ['ข้าว']]
```

Like the Ruby side, every entry point hardens its input (transcodes non–UTF-8, scrubs invalid
bytes, strips NULs), so segmentation never crashes on malformed text.

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

For the full pipeline — the multi-granularity lattice, the interned KN-bigram model, the OOV
back-off and branching-entropy merge, and the decoupled word/syllable Viterbi passes — see
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), with an end-to-end diagram.

## Benchmarks

On the research-standard `pythainlp.benchmarks` metric, the shipped LST20 model posts the highest
word-level F1 on 4 of 5 Thai corpora (LST20, VISTEC, TNHC, WS1000) and is close on BEST — though
part of that margin is a home-corpus advantage, since the neural baselines are BEST-trained and run
out-of-domain on the other four (the [benchmarks doc](docs/BENCHMARKS.md) reads each tool on its
home turf too). What's less ambiguous is the cost: it reaches that accuracy at dictionary-class
speed, ~27× faster than attacut and ~740× faster than deepcut. Its weak spot is recall on
out-of-vocabulary words, where the neural models lead. Full tables, methodology, caveats, and
reproduction steps are in [docs/BENCHMARKS.md](docs/BENCHMARKS.md).

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
  ShareAlike) — and from the **LST20** corpus (NECTEC non-commercial; see below).
- The n-gram language model is trained on the **LST20** corpus (which also contributes dictionary
  vocabulary) — NECTEC's agreement permits
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
