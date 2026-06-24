# test/test_std_normalize.rb
# Locks down Thapthim.std_normalize — the FFI wrapper over the Rust normalizer. We assert the
# documented transformations (spaces, zero-width, vowel/tone reorder, dangling marks), plus the
# robustness contract it inherits from sanitize_input (nil/empty/non-String/encoding/NUL) and the
# output guarantee (valid UTF-8, NUL-free). Idempotence is checked because the normalizer is meant
# to be a fixed point: re-normalizing already-clean text must be a no-op.
require "minitest/autorun"
require_relative "../lib/thapthim"

class TestStdNormalize < Minitest::Test
  NUL = 0.chr

  # --- whitespace -----------------------------------------------------------------

  def test_collapses_and_trims_spaces
    assert_equal "ฉัน กิน", Thapthim.std_normalize("  ฉัน   กิน  ")
  end

  def test_collapses_newlines
    assert_equal "ก\nข", Thapthim.std_normalize("ก  \n \n  ข")
  end

  # --- zero-width characters ------------------------------------------------------

  def test_strips_zero_width_chars
    # U+200B ZERO WIDTH SPACE, U+200C ZERO WIDTH NON-JOINER
    assert_equal "กข", Thapthim.std_normalize("ก​ข‌")
  end

  # --- vowel / tone reordering ----------------------------------------------------

  def test_merges_double_sara_e
    # เ + เ  =>  แ
    assert_equal "แก", Thapthim.std_normalize("เเก")
  end

  def test_does_not_alter_already_normalized_text
    clean = "ฉันกินข้าวที่ร้านอาหารไทย"
    assert_equal clean, Thapthim.std_normalize(clean)
  end

  # --- idempotence: normalize is a fixed point ------------------------------------

  def test_is_idempotent
    [
      "  ฉัน   กิน  ",
      "เเก",
      "ก​ข",
      "ก  \n \n  ข",
      "ฉันรักภาษาไทย",
      "iPhone15 ราคา ดี",
    ].each do |s|
      once = Thapthim.std_normalize(s)
      twice = Thapthim.std_normalize(once)
      assert_equal once, twice, "std_normalize must be idempotent for #{s.inspect}"
    end
  end

  # --- trivial / non-String input -------------------------------------------------

  def test_nil_passes_through
    assert_nil Thapthim.std_normalize(nil)
  end

  def test_empty_returns_empty
    assert_equal "", Thapthim.std_normalize("")
  end

  def test_non_string_is_coerced
    assert_equal "12345", Thapthim.std_normalize(12345)
  end

  def test_frozen_input_is_accepted
    assert_equal "ฉัน กิน", Thapthim.std_normalize("  ฉัน   กิน  ".freeze)
  end

  def test_does_not_mutate_input
    raw = "  ฉัน   กิน  ".dup
    before = raw.dup
    Thapthim.std_normalize(raw)
    assert_equal before, raw
  end

  # --- robustness inherited from sanitize_input -----------------------------------

  def test_embedded_nul_is_handled
    result = Thapthim.std_normalize("ฉัน#{NUL}กิน")
    refute_includes result, NUL, "output must be NUL-free"
    assert_includes result, "กิน", "text after the NUL must survive"
  end

  def test_invalid_utf8_is_scrubbed_not_crashed
    text = ("ฉัน".b + "\xFF\xFE".b).force_encoding("UTF-8")
    refute text.valid_encoding?
    result = Thapthim.std_normalize(text)
    assert result.valid_encoding?
    assert_includes result, "ฉัน"
  end

  def test_tis620_is_transcoded
    text = "\xA1\xD2\xC3".dup.force_encoding("TIS-620") # "การ"
    assert_equal "การ", Thapthim.std_normalize(text)
  end

  # --- output contract ------------------------------------------------------------

  def test_output_is_utf8_and_valid
    [nil, "", "ฉัน", "  ก  ข  ", "เเก", 999].each do |s|
      out = Thapthim.std_normalize(s)
      next if out.nil?
      assert_equal Encoding::UTF_8, out.encoding, "for #{s.inspect}"
      assert out.valid_encoding?, "for #{s.inspect}"
    end
  end
end
