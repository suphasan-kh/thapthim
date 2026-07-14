# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0

# test/test_collate.rb
# Locks down Thai dictionary collation (Royal Society of Thailand order): the two rules that native
# codepoint sorting gets wrong — leading-vowel reordering (เ แ โ ใ ไ sort after their consonant) and
# tone marks as a secondary level (ปา < ป่า < ป้า) — plus the block form, mixed non-Thai handling,
# and the sort-key contract (comparable, storable, deterministic).
require "minitest/autorun"
require_relative "../lib/thapthim"

class TestCollate < Minitest::Test
  def test_leading_vowels_sort_after_their_consonant
    # native sort gives [กา, ขา, เกา, ไก่, ไข่] — ขา wrongly splits the ก-group.
    input = ["ไก่", "กา", "ขา", "ไข่", "เกา"]
    assert_equal ["กา", "เกา", "ไก่", "ขา", "ไข่"], Thapthim.thai_sort(input)
  end

  def test_tone_marks_are_secondary
    assert_equal ["ปา", "ป่า", "ป้า"], Thapthim.thai_sort(["ป้า", "ปา", "ป่า"])
  end

  def test_differs_from_native_sort
    input = ["ไก่", "กา", "ขา"]
    refute_equal input.sort, Thapthim.thai_sort(input) # native is wrong here; ours must differ
    assert_equal ["กา", "ไก่", "ขา"], Thapthim.thai_sort(input)
  end

  def test_block_form_sorts_objects_by_key
    user = Struct.new(:name)
    users = [user.new("ไก่"), user.new("กา"), user.new("ขา")]
    assert_equal ["กา", "ไก่", "ขา"], Thapthim.thai_sort(users) { |u| u.name }.map(&:name)
  end

  def test_non_thai_sorts_after_thai
    assert_equal ["กา", "ไก่", "apple", "banana"], Thapthim.thai_sort(["banana", "ไก่", "apple", "กา"])
  end

  def test_sort_key_is_comparable_storable_and_deterministic
    ka  = Thapthim.thai_sort_key("กา")
    kai = Thapthim.thai_sort_key("ไก่")
    assert ka < kai, "key order must match sort order"
    assert_equal Encoding::BINARY, ka.encoding, "key is a storable binary string"
    assert_equal ka, Thapthim.thai_sort_key("กา"), "same input → identical key"
  end

  def test_sort_key_matches_thai_sort
    words = ["ไก่", "กา", "ขา", "ไข่", "เกา", "ป่า", "ปา"]
    assert_equal Thapthim.thai_sort(words), words.sort_by { |w| Thapthim.thai_sort_key(w) }
  end

  # Cases derived directly from the Royal Institute rules
  # (https://th.wikibooks.org/wiki/วิธีเรียงลำดับคำตามตัวอักษรในภาษาไทย):

  def test_rule_no_vowel_word_precedes_voweled_word
    assert_equal ["กก", "กะ"], Thapthim.thai_sort(["กะ", "กก"]) # กก before กะ
  end

  def test_rule_rue_sorts_between_ro_and_lo
    assert_equal ["รวย", "ฤดู", "ลม"], Thapthim.thai_sort(["ลม", "ฤดู", "รวย"]) # ร < ฤ < ล
  end

  def test_rule_mai_han_akat_sorts_before_sara_aa
    assert_equal ["กัน", "กาก"], Thapthim.thai_sort(["กาก", "กัน"]) # ั before า → all กั- before all กา-
  end

  def test_rule_vowel_sequence_order
    assert_equal ["กา", "กำ", "กิ"], Thapthim.thai_sort(["กิ", "กำ", "กา"]) # า ำ ิ
  end

  def test_rule_consonant_form_paramount_over_tone
    assert_equal ["กก", "ก่ะ"], Thapthim.thai_sort(["ก่ะ", "กก"]) # base letters decide, not the tone mark
  end

  def test_handles_empty_and_hardened_input
    assert_equal ["", "กา"], Thapthim.thai_sort(["กา", ""])
    # nil is hardened to "" (not raised), and an empty string sorts first.
    assert_equal Thapthim.thai_sort_key(""), Thapthim.thai_sort_key(nil)
  end
end
