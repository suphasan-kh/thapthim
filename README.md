# Thapthim (ทับทิม)

Thai natural language processing for Ruby (and Python), backed by a native Rust engine.

Ruby has no comprehensive Thai NLP toolkit the way Python has [PyThaiNLP](https://pythainlp.org) —
existing gems are single-task word-segmentation wrappers around C or Rust engines, most of them
unmaintained. Thapthim aims to fill that gap: one gem for processing Thai text. Because Thai is written without
spaces between words, almost every downstream task starts with **segmentation** — so that is the
foundation Thapthim ships first, alongside text normalization. Further tasks are planned on the
same engine (see [Roadmap](#roadmap)).

```ruby
Thapthim.word_segment("ฉันกินข้าว")      # => ["ฉัน", "กิน", "ข้าว"]
Thapthim.syllable_segment("ฉันกินข้าว")  # => ["ฉัน", "กิน", "ข้าว"]
```

## Capabilities

Shipping today:

- **Word segmentation** (`word_segment`) — dictionary-lattice decoding over Thai Character Cluster
  (TCC) boundaries, scored with a Kneser-Ney bigram language model via Viterbi, with a
  branching-entropy back-off for out-of-vocabulary spans.
- **Syllable segmentation** (`syllable_segment`) — orthographic syllables from an independent
  syllable-level pass on the same grid.
- **TCC segmentation** (`tcc_segment`) — the smallest orthographically inseparable units.
- **Part-of-speech tagging** (`pos_tag`) — a standard first-order HMM over the 16-tag LST20 tagset.
  Tags a token array, or a string it segments first (cascade).
- **Text normalization** (`std_normalize`) — whitespace collapsing, vowel/tone-mark reordering,
  zero-width–character stripping, repeated/dangling-mark cleanup (orthographic-level correction).
- **Number ↔ Thai text** (`num2text`, `baht_text`, `read_digits`, `text2num`) — read a number as Thai
  cardinal words, as baht/satang currency text (`baht_text` rounds to two satang digits and appends
  ถ้วน for whole amounts), or digit-by-digit (`read_digits`, for phone numbers / PINs / years);
  `text2num` parses Thai number words back to a numeric value. The readers accept an Integer, Float,
  or numeric string.
- **Numeral & era conversion** (`thai2arabic_digits` / `arabic2thai_digits`, `be2ce` / `ce2be`) —
  Thai digits ๐–๙ ↔ Arabic 0–9 within a string, and Buddhist ↔ Common era years (offset 543).
- **Thai collation / sort** (`thai_sort`, `thai_sort_key`) — ⚠️ **experimental, not yet verified** —
  dictionary-order sorting per the Royal Society of Thailand (leading vowels เ แ โ ใ ไ reorder after
  their consonant; tone marks secondary). Native Ruby sorts Thai by codepoint, which is wrong.
  `thai_sort_key` returns a comparable, storable key — use it with `sort_by`/`min_by`, or persist it
  in a database column to `ORDER BY` Thai correctly. The numeral and collation helpers above are pure
  Ruby and not yet exposed in Python.
- **Spelling correction** (`spell`, `suggest`, `correct`, `correct_sent`) — noisy-channel correction
  over the PyThaiNLP `thai_words` dictionary: candidates within a length-adaptive Damerau-Levenshtein
  bound (trie search), ranked by word frequency, or by bigram context for whole sentences
  (`correct_sent`). Frequencies are `thai_words`-aligned; valid words are left unchanged. Ruby-only
  for now.
- **Robust input handling** — non–UTF-8 (e.g. TIS-620) transcoding, invalid-byte and NUL scrubbing.

## Roadmap

Planned next capabilities, in priority order. The sequence-labeling tasks plug into the
task-agnostic Viterbi lattice core (see [Extensibility](docs/ARCHITECTURE.md#extensibility)) as a
candidate set plus a cost model; the rest are deterministic transforms beside it.

- **Sentence segmentation** — Thai marks sentence breaks with spaces, ambiguously (a space is
  also a phrase/clause separator); a boundary classifier over the segmented word stream,
  trainable from LST20's sentence layer.
- **Transliteration / romanization** — deterministic script transforms (e.g. ISO 11940).

APIs for these will follow the same shape as segmentation: module-level functions, hardened input,
identical behavior from Ruby and Python.

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

# Whitespace-only tokens are kept by default (matching PyThaiNLP), so the tokens reassemble
# losslessly into the input (tokens.join == text); pass keep_whitespace: false to drop them.
Thapthim.word_segment("ฉัน กิน")                           # => ["ฉัน", " ", "กิน"]
Thapthim.word_segment("ฉัน กิน", keep_whitespace: false)   # => ["ฉัน", "กิน"]

# Optional normalization before segmenting (collapses spaces, reorders vowels, strips
# zero-width chars, removes repeated marks).
Thapthim.word_segment("  ฉัน   กิน  ", normalize: true)
Thapthim.std_normalize("  ฉัน   กิน  ")    # => "ฉัน กิน"

# Thai Character Clusters — the lowest-level inseparable units.
Thapthim.tcc_segment("ฉันกินข้าว")         # => ["ฉั", "น", "กิ", "น", "ข้า", "ว"]

# Part-of-speech tagging (LST20 16-tag set) → [surface, tag] pairs. A string is segmented first
# (cascade); pass an array to tag already-segmented tokens directly.
Thapthim.pos_tag("ฉันกินข้าว")             # => [["ฉัน", "PR"], ["กิน", "VV"], ["ข้าว", "NN"]]
Thapthim.pos_tag(["ฉัน", "กิน", "ข้าว"])   # tag gold tokens directly (isolates the tagger)

# Numbers to Thai text. num2text reads the value; a fractional part is spoken digit-by-digit after
# จุด. baht_text reads it as currency (บาท/สตางค์, ...ถ้วน when whole). Pass a String to keep exact
# digits — a Float has already dropped trailing zeros (3.50 is 3.5).
Thapthim.num2text(2568)          # => "สองพันห้าร้อยหกสิบแปด"
Thapthim.num2text("3.50")        # => "สามจุดห้าศูนย์"
Thapthim.baht_text(1234.5)       # => "หนึ่งพันสองร้อยสามสิบสี่บาทห้าสิบสตางค์"
Thapthim.read_digits("081-234")  # => "ศูนย์แปดหนึ่งสองสามสี่"  (each digit; keeps the leading zero)
Thapthim.text2num("สองพันห้าร้อยหกสิบแปด")  # => 2568  (Integer; Float when a จุด is present)

# Digit and era conversion.
Thapthim.thai2arabic_digits("พ.ศ. ๒๕๖๘")   # => "พ.ศ. 2568"
Thapthim.arabic2thai_digits("2568")         # => "๒๕๖๘"
Thapthim.be2ce(2568)                        # => 2025  (Buddhist → Common era)
Thapthim.ce2be(2025)                        # => 2568  (Common → Buddhist era)

# Thai dictionary sort (native sort is wrong: it puts เป็ด/ไก่ last, by codepoint).
Thapthim.thai_sort(["ไก่", "กา", "ขา"])     # => ["กา", "ไก่", "ขา"]
Thapthim.thai_sort(users) { |u| u.name }    # sort objects by a Thai field
# thai_sort_key(name) is a comparable binary key: use with sort_by/min_by, or store it in an
# indexed column and ORDER BY it so the database returns Thai in dictionary order.

# Spelling: spell (is it a word?), suggest (ranked candidates), correct (best fix; valid words are
# left unchanged), correct_sent (context-aware over a sentence, using the bigram).
Thapthim.spell("ประเทด")                 # => false
Thapthim.suggest("ประเทด")               # => ["ประเทศ", ...]  (ranked)
Thapthim.correct("ผลไม")                 # => "ผลไม้"
# correct_sent takes a String (segmented first) or a token array; the bigram picks the right word
# in context. (Pass tokens when you already have them — a String relies on segmentation.)
Thapthim.correct_sent(["เขา", "มี", "ความสามาด"])  # => ["เขา", "มี", "ความสามารถ"]
```

Every entry point hardens its input — non–UTF-8 (e.g. TIS-620) is transcoded, invalid bytes and
embedded NULs are scrubbed — so segmentation never crashes on malformed text.

## Python

The same Rust engine is exposed to Python via [PyO3](https://pyo3.rs)/[maturin](https://www.maturin.rs);
the API mirrors Ruby (identical engine, assets, results). Needs a Rust toolchain and Python ≥ 3.8.

```bash
pip install .                       # build + install from a clone (simplest)
# editable workflow (needs a venv):
pip install 'maturin>=1.9,<2.0' && maturin develop --release
```

```python
import thapthim
thapthim.word_segment("ฉันกินข้าว")               # ['ฉัน', 'กิน', 'ข้าว']
thapthim.syllable_segment("ฉันกินข้าว")
thapthim.word_segment("ฉัน กิน", keep_whitespace=False)  # whitespace kept by default
thapthim.word_segment("  ฉัน  ", normalize=True)
thapthim.tcc_segment("ฉันกินข้าว")
thapthim.pos_tag("ฉันกินข้าว")                    # [('ฉัน', 'PR'), ('กิน', 'VV'), ('ข้าว', 'NN')]
thapthim.pos_tag(["ฉัน", "กิน", "ข้าว"])          # tag already-segmented tokens
thapthim.word_segment_offsets("ฉันกิน")           # [(0, 9), (9, 9)]  (start_byte, length)
thapthim.word_segment_batch(["ฉันกิน", "ข้าว"])   # bulk: releases the GIL, fans across cores
```

On **Google Colab** (no Rust by default) install the toolchain first, then build from git:

```python
!curl https://sh.rustup.rs -sSf | sh -s -- -y
import os; os.environ["PATH"] += ":" + os.path.expanduser("~/.cargo/bin")
!pip install "git+https://github.com/suphasan-kh/thapthim.git"   # ~1–3 min to compile
```

## How segmentation works

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
