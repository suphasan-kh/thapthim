# test/test_input_hardening.rb
# Documents and locks down the input-robustness contract of the public segmentation API:
# arbitrary/dirty input must never crash, panic, silently truncate, or vanish entirely.
require "minitest/autorun"
require_relative "../lib/thapthim"

class TestInputHardening < Minitest::Test
  NUL = 0.chr

  # --- trivial / non-String input -------------------------------------------------

  def test_empty_and_nil
    assert_equal [], Thapthim.segment("")
    assert_equal [], Thapthim.segment(nil)
    assert_equal [], Thapthim.syllables("")
    assert_equal [], Thapthim.syllables(nil)
    assert_equal [], Thapthim.tcc_segment("")
    assert_equal [], Thapthim.tcc_segment(nil)
  end

  def test_non_string_input_is_coerced_not_raised
    assert_equal ["12345"], Thapthim.segment(12345)
    refute_empty Thapthim.tcc_segment(12345)
  end

  def test_frozen_string_is_accepted
    assert_equal ["ฉัน", "รัก"], Thapthim.segment("ฉันรัก".freeze)
  end

  # --- embedded NUL: must not truncate at the NUL ---------------------------------

  def test_embedded_nul_does_not_truncate
    text = "ฉัน" + NUL + "รัก"
    result = Thapthim.segment(text)
    assert_includes result, "รัก", "text after the NUL must survive"
    assert_equal "ฉันรัก", result.join
    refute(result.any? { |t| t.include?(NUL) }, "no token may contain a NUL")
  end

  def test_embedded_nul_in_tcc_path
    text = "กา" + NUL + "ไก่"
    refute_empty Thapthim.tcc_segment(text)
    assert_equal "กาไก่", Thapthim.tcc_segment(text).join
  end

  # --- invalid UTF-8: scrub bad bytes, keep the valid surroundings ----------------

  def test_invalid_utf8_keeps_valid_text
    text = ("ฉัน".b + "\xFF\xFE".b + "รัก".b).force_encoding("UTF-8")
    refute text.valid_encoding?, "fixture must actually be invalid UTF-8"
    result = Thapthim.segment(text)
    assert_includes result, "ฉัน"
    assert_includes result, "รัก"
    assert(result.all? { |t| t.encoding == Encoding::UTF_8 && t.valid_encoding? },
           "every returned token must be valid UTF-8")
  end

  # --- non-UTF-8 encodings: transcode rather than drop ----------------------------

  def test_tis620_is_transcoded
    # 0xA1 0xD2 0xC3 in TIS-620 == "การ"
    text = "\xA1\xD2\xC3".dup.force_encoding("TIS-620")
    assert_equal ["การ"], Thapthim.segment(text)
  end

  def test_binary_utf8_bytes_are_reinterpreted
    text = "\xE0\xB8\x81".dup.force_encoding("ASCII-8BIT") # UTF-8 bytes for "ก"
    assert_equal ["ก"], Thapthim.segment(text)
  end

  # --- output contract ------------------------------------------------------------

  def test_output_tokens_are_valid_utf8
    Thapthim.segment("iPhone15ราคาดี😀").each do |t|
      assert_equal Encoding::UTF_8, t.encoding
      assert t.valid_encoding?
    end
  end

  def test_deterministic_for_identical_input
    s = "ฉันนั่งตากลม จากนั้นไปชมกอดอกไม้หลังบ้าน"
    assert_equal Thapthim.segment(s), Thapthim.segment(s)
  end

  # --- Thai-numeral grouping (years / phone / time stay whole) --------------------

  def test_thai_numerals_group_as_one_token
    assert_includes Thapthim.segment("ปี ๒๕๖๙"), "๒๕๖๙"
    assert_includes Thapthim.segment("เบอร์ ๐๘๑๒๓๔๕๖๗๘"), "๐๘๑๒๓๔๕๖๗๘"
    # interior separator (time / decimal) is kept whole, matching LST20's ASCII convention
    assert_includes Thapthim.segment("เวลา ๑๔.๓๐ น."), "๑๔.๓๐"
  end

  def test_large_input_does_not_crash
    big = "ฉันรักภาษาไทย" * 5000
    assert_equal 20_000, Thapthim.segment(big).length
  end

  # --- the original motivating sentence no longer produces the bogus กอดอก token --
  # (the OOV-penalty fix splits the run as กอ|ดอก|ไม้ rather than gluing กอดอก).

  def test_known_oov_sentence
    result = Thapthim.segment("ฉันนั่งตากลม จากนั้นไปชมกอดอกไม้หลังบ้าน")
    refute_includes result, "กอดอก", "the dictionary-junk over-merge must not reappear"
    assert_includes result, "กอ"
    assert_includes result, "ดอก"
  end
end
