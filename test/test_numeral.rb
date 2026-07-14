# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0

# test/test_numeral.rb
# Locks down the deterministic numeral transforms: era year conversion (be2ce/ce2be) and Thai/Arabic
# digit transliteration. The era pair must round-trip and use the fixed 543 offset; the digit pair
# must convert only digit characters (leaving Thai text, punctuation, and separators intact), round-
# trip, and inherit the sanitize_input hardening (nil/empty/non-String/encoding).
require "minitest/autorun"
require_relative "../lib/thapthim"

class TestNumeral < Minitest::Test
  # --- era conversion -------------------------------------------------------------

  def test_be2ce_known_year
    assert_equal 2025, Thapthim.be2ce(2568)
  end

  def test_ce2be_known_year
    assert_equal 2568, Thapthim.ce2be(2025)
  end

  def test_era_round_trips
    assert_equal 2025, Thapthim.be2ce(Thapthim.ce2be(2025))
  end

  def test_era_accepts_integer_string
    assert_equal 2025, Thapthim.be2ce("2568")
  end

  def test_era_rejects_non_integer
    assert_raises(ArgumentError) { Thapthim.be2ce("not a year") }
    assert_raises(TypeError)     { Thapthim.ce2be(nil) }
  end

  # --- digit transliteration ------------------------------------------------------

  def test_thai_to_arabic_digits
    assert_equal "2568", Thapthim.thai2arabic_digits("๒๕๖๘")
  end

  def test_arabic_to_thai_digits
    assert_equal "๒๕๖๘", Thapthim.arabic2thai_digits("2568")
  end

  def test_digits_convert_only_digits_leaving_text_intact
    # Thai letters, a period, and spaces must pass through untouched.
    assert_equal "พ.ศ. 2568 ปี", Thapthim.thai2arabic_digits("พ.ศ. ๒๕๖๘ ปี")
  end

  def test_digits_round_trip
    assert_equal "๐๑๒๓๔๕๖๗๘๙", Thapthim.arabic2thai_digits(Thapthim.thai2arabic_digits("๐๑๒๓๔๕๖๗๘๙"))
  end

  def test_digits_compose_with_era_conversion
    # A common real use: read a Thai-digit BE year out of text and get the CE integer.
    be_year = Integer(Thapthim.thai2arabic_digits("๒๕๖๘"))
    assert_equal 2025, Thapthim.be2ce(be_year)
  end

  # --- input hardening (inherited from sanitize_input) ----------------------------

  def test_digits_handle_nil_and_empty
    assert_equal "", Thapthim.thai2arabic_digits(nil)
    assert_equal "", Thapthim.arabic2thai_digits("")
  end

  def test_digits_handle_non_string
    assert_equal "2568", Thapthim.arabic2thai_digits(2568).then { |thai| Thapthim.thai2arabic_digits(thai) }
  end

  # --- num2text: cardinal reading + irregularities --------------------------------

  def test_num2text_digits_and_zero
    assert_equal "ศูนย์", Thapthim.num2text(0)
    assert_equal "เก้า",  Thapthim.num2text(9)
  end

  def test_num2text_tens_irregulars
    assert_equal "สิบ",        Thapthim.num2text(10)   # tens-1 is bare สิบ
    assert_equal "สิบเอ็ด",     Thapthim.num2text(11)   # units-1 after a higher place → เอ็ด
    assert_equal "ยี่สิบ",      Thapthim.num2text(20)   # tens-2 is ยี่สิบ
    assert_equal "ยี่สิบเอ็ด",  Thapthim.num2text(21)
    assert_equal "สามสิบห้า",   Thapthim.num2text(35)
  end

  def test_num2text_hundreds_keep_neung
    # ร้อย keeps หนึ่ง (only สิบ drops it), and a units-1 after it is still เอ็ด.
    assert_equal "หนึ่งร้อย",     Thapthim.num2text(100)
    assert_equal "หนึ่งร้อยเอ็ด", Thapthim.num2text(101)
  end

  def test_num2text_year_and_large
    assert_equal "สองพันห้าร้อยหกสิบแปด", Thapthim.num2text(2568)
    assert_equal "หนึ่งล้าน",             Thapthim.num2text(1_000_000)
    assert_equal "หนึ่งล้านเอ็ด",         Thapthim.num2text(1_000_001) # เอ็ด carries across ล้าน
    assert_equal "หนึ่งล้านล้าน",         Thapthim.num2text(1_000_000_000_000)
  end

  def test_num2text_decimal_reads_digit_by_digit
    assert_equal "สามจุดหนึ่งสี่",   Thapthim.num2text(3.14)
    assert_equal "ศูนย์จุดห้า",       Thapthim.num2text(0.5)
    assert_equal "สามจุดศูนย์ห้า",    Thapthim.num2text("3.05") # leading frac zero is spoken
  end

  def test_num2text_keeps_every_digit_including_trailing_zeros
    assert_equal "สามจุดศูนย์",       Thapthim.num2text(3.0)      # a .0 float keeps its decimal
    assert_equal "สามจุดห้าศูนย์",     Thapthim.num2text("3.50")   # trailing zero is spoken, verbatim
    assert_equal "สาม",              Thapthim.num2text(3)        # a plain Integer has no decimal
  end

  def test_num2text_negative_and_string_and_thai_digits
    assert_equal "ลบห้า",                 Thapthim.num2text(-5)
    assert_equal "สองพันห้าร้อยหกสิบแปด", Thapthim.num2text("๒๕๖๘") # Thai-digit string
  end

  # --- baht_text: currency wrapper ------------------------------------------------

  def test_baht_text_whole_is_thuan
    assert_equal "หนึ่งบาทถ้วน",    Thapthim.baht_text(1)
    assert_equal "ศูนย์บาทถ้วน",    Thapthim.baht_text(0)
  end

  def test_baht_text_with_satang
    assert_equal "สามบาทห้าสิบสตางค์", Thapthim.baht_text(3.5)
    assert_equal "สามบาทสิบสี่สตางค์", Thapthim.baht_text(3.14)
  end

  def test_baht_text_satang_only_omits_baht
    assert_equal "ห้าสิบสตางค์", Thapthim.baht_text(0.5)
  end

  def test_baht_text_string_rounds_exactly
    # String input avoids Float error: 3.145 → 3.15 → 15 สตางค์.
    assert_equal "สามบาทสิบห้าสตางค์", Thapthim.baht_text("3.145")
  end

  def test_baht_text_negative
    assert_equal "ลบสามบาทห้าสิบสตางค์", Thapthim.baht_text(-3.5)
  end

  def test_num2text_rejects_non_number
    assert_raises(ArgumentError) { Thapthim.num2text("abc") }
    assert_raises(ArgumentError) { Thapthim.num2text(nil) }
  end
end
