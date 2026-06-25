# test/test_segment.rb
# Behavioural + invariant tests for the word/syllable segmentation API
# (Thapthim.word_segment / Thapthim.syllable_segment). The input-hardening contract lives in
# test_input_hardening.rb; this file pins down the *correctness* properties:
#   - losslessness   : tokens reassemble exactly into the (sanitized) input
#   - syllable nesting: word boundaries are a strict subset of syllable boundaries
#   - output contract : always an Array of valid-UTF-8, NUL-free Strings
#   - determinism     : identical input -> identical output
#   - normalize:       : tokens are substrings of the *normalized* text, input untouched
require "minitest/autorun"
require_relative "../lib/thapthim"

class TestSegment < Minitest::Test
  NUL = 0.chr

  # A deliberately varied corpus: pure Thai, Thai+Latin+digits, Thai numerals,
  # punctuation/whitespace, emoji (astral), combining sequences, and mixed scripts.
  # Every string here is already clean UTF-8, so the sanitized form == the input and
  # the lossless invariant can assert join == input directly.
  CORPUS = [
    "ฉันกินข้าว",
    "ฉันรักภาษาไทย",
    "ทนายปีศาจ เนี่ยเริ่มต้นจากแซมกับซันและก็ทีมเขียนบท",
    "iPhone15ราคาดี😀",
    "ปี ๒๕๖๙ เวลา ๑๔.๓๐ น.",
    "เบอร์ ๐๘๑๒๓๔๕๖๗๘ โทรมาได้",
    "Hello world",
    "ก\nข\tค",
    "   ",
    "ก่่่่่ข",                 # combining-mark overload
    "👨‍👩‍👧 ครอบครัว",         # ZWJ emoji sequence
    "ราคา 1,234.56 บาท",
    "ฉันนั่งตากลม จากนั้นไปชมกอดอกไม้หลังบ้าน",
  ].freeze

  # --- basic, human-checkable segmentations ---------------------------------------

  def test_basic_word_segmentation
    assert_equal ["ฉัน", "กิน", "ข้าว"], Thapthim.word_segment("ฉันกินข้าว")
  end

  def test_returns_array_of_strings
    result = Thapthim.word_segment("ฉันกินข้าว")
    assert_kind_of Array, result
    assert(result.all? { |t| t.is_a?(String) })
  end

  def test_syllables_basic
    refute_empty Thapthim.syllable_segment("ฉันกินข้าว")
    assert(Thapthim.syllable_segment("ฉันกินข้าว").all? { |t| t.is_a?(String) })
  end

  # --- losslessness: nothing is dropped, duplicated, or reordered -----------------

  def test_segment_is_lossless
    CORPUS.each do |s|
      assert_equal s, Thapthim.word_segment(s).join,
                   "segment must reconstruct the input exactly: #{s.inspect}"
    end
  end

  def test_syllables_is_lossless
    CORPUS.each do |s|
      assert_equal s, Thapthim.syllable_segment(s).join,
                   "syllables must reconstruct the input exactly: #{s.inspect}"
    end
  end

  def test_tokens_are_non_empty
    CORPUS.each do |s|
      refute_includes Thapthim.word_segment(s), "", "no empty word token for #{s.inspect}"
      refute_includes Thapthim.syllable_segment(s), "", "no empty syllable token for #{s.inspect}"
    end
  end

  # --- syllable boundaries are a superset of word boundaries ----------------------
  # Every word break must coincide with a syllable break (documented contract).

  def test_word_boundaries_are_subset_of_syllable_boundaries
    CORPUS.each do |s|
      word_bounds = cumulative_byte_offsets(Thapthim.word_segment(s))
      syl_bounds  = cumulative_byte_offsets(Thapthim.syllable_segment(s))
      missing = word_bounds - syl_bounds
      assert_empty missing,
                   "word boundaries must be syllable boundaries (#{s.inspect}); orphaned at #{missing.inspect}"
    end
  end

  # --- output contract ------------------------------------------------------------

  def test_every_token_is_valid_utf8_and_nul_free
    CORPUS.each do |s|
      (Thapthim.word_segment(s) + Thapthim.syllable_segment(s)).each do |t|
        assert_equal Encoding::UTF_8, t.encoding, "token encoding for #{s.inspect}"
        assert t.valid_encoding?, "token must be valid UTF-8 for #{s.inspect}: #{t.inspect}"
        refute_includes t, NUL, "token must not contain NUL for #{s.inspect}"
      end
    end
  end

  # --- determinism ----------------------------------------------------------------

  def test_segment_is_deterministic
    CORPUS.each do |s|
      assert_equal Thapthim.word_segment(s), Thapthim.word_segment(s), "segment unstable for #{s.inspect}"
      assert_equal Thapthim.syllable_segment(s), Thapthim.syllable_segment(s), "syllables unstable for #{s.inspect}"
    end
  end

  # --- whitespace / control chars are preserved verbatim --------------------------

  def test_whitespace_and_controls_preserved
    assert_equal "   ", Thapthim.word_segment("   ").join
    assert_equal "ก\nข\tค", Thapthim.word_segment("ก\nข\tค").join
  end

  # --- astral / combining sequences survive byte-for-byte -------------------------

  def test_astral_and_combining_sequences_preserved
    %W[😀 👨‍👩‍👧 ก่่่].each do |s|
      assert_equal s, Thapthim.word_segment(s).join
      assert_equal s, Thapthim.syllable_segment(s).join
    end
  end

  # --- normalize: true ------------------------------------------------------------

  def test_normalize_tokens_reassemble_into_normalized_text
    raw = "  ฉัน   กิน  "
    normalized = Thapthim.std_normalize(raw)
    assert_equal normalized, Thapthim.word_segment(raw, normalize: true).join
    assert_equal normalized, Thapthim.syllable_segment(raw, normalize: true).join
  end

  def test_normalize_does_not_mutate_input
    raw = "  ฉัน   กิน  ".dup
    before = raw.dup
    Thapthim.word_segment(raw, normalize: true)
    Thapthim.syllable_segment(raw, normalize: true)
    assert_equal before, raw, "normalize: true must not mutate the caller's string"
  end

  def test_normalize_false_is_the_default
    raw = "  ฉัน   กิน  "
    assert_equal Thapthim.word_segment(raw), Thapthim.word_segment(raw, normalize: false)
    refute_equal Thapthim.word_segment(raw, normalize: true), Thapthim.word_segment(raw, normalize: false)
  end

  def test_frozen_input_with_normalize
    assert_equal ["ฉัน", "รัก"], Thapthim.word_segment("ฉันรัก".freeze, normalize: true)
  end

  # --- caller may freely mutate returned tokens (they are private copies) ----------

  def test_returned_tokens_are_independent_copies
    first = Thapthim.word_segment("ฉันกินข้าว")
    first.first << "XXX" rescue nil
    second = Thapthim.word_segment("ฉันกินข้าว")
    assert_equal ["ฉัน", "กิน", "ข้าว"], second,
                 "mutating a returned token must not corrupt later results"
  end

  private

  # Cumulative byte offset after each token — the set of boundary positions.
  def cumulative_byte_offsets(tokens)
    acc = 0
    tokens.map { |t| acc += t.bytesize }
  end
end
