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

  def test_rule_mai_han_akat_is_a_primary_vowel_with_composite
    # ั is the 2nd vowel in RI rule 2 (verbatim), incl. its composite ัวะ (the page's ผัวะ example).
    assert_equal ["ผัด", "ผัว", "ผัวะ", "ผา"], Thapthim.thai_sort(["ผา", "ผัวะ", "ผัด", "ผัว"])
  end

  def test_rule_vowel_sequence_order
    assert_equal ["กา", "กำ", "กิ"], Thapthim.thai_sort(["กิ", "กำ", "กา"]) # า ำ ิ
  end

  def test_rule_consonant_form_paramount_over_tone
    assert_equal ["กก", "ก่ะ"], Thapthim.thai_sort(["ก่ะ", "กก"]) # base letters decide, not the tone mark
  end

  def test_all_vowel_patterns_sort_in_ri_order
    # The complete RI rule-2 vowel-form order: every base vowel AND every composite the rule lists
    # (of ั, เ, แ, โ), realized on one consonant so the vowel form is what sorts. Composites fall out
    # of character-by-character comparison — no special-casing.
    ordered = %w[
      กะ กัน กัวะ กา กำ กิ กี กึ กือ กุ กู
      เก เกะ เกา เกาะ เกิน เกีย เกียะ เกือ เกือะ
      แก แกะ โก โกะ ใก ไก
    ]
    assert_equal ordered, Thapthim.thai_sort(ordered.reverse)
  end

  def test_yo_wo_o_count_as_consonants
    # RI rule: ย ว อ are always consonants. เสือ ends in อ (consonant); sorts by vowel forms า < ี < ื.
    assert_equal ["เสา", "เสีย", "เสือ"], Thapthim.thai_sort(["เสือ", "เสีย", "เสา"])
  end

  # Every worked ordering example from the canonical RI rules text. Each pair (a, b) asserts a < b.
  def test_ri_rules_worked_examples
    pairs = [
      %w[กลอน คลอน],          # ก < ค
      %w[ศาลา สาระ],          # ศ < ส
      %w[จักรพรรณ จักรพรรดิ],   # 8th char: ณ < ด
      %w[แกลบ ครอง],          # initial ก < ค
      %w[ไกล เพลง],           # initial ก < พ
      %w[เกวียน เกิน],         # 3rd: ว (consonant) < สระอิ
      %w[เกวียน ไกล],          # 2nd: เ < ไ
      %w[เกม แกง],            # เ < แ
      %w[เกเร เกลอ],          # 3rd: ร < ล
      %w[สีแดง แสดง],         # สระอี < แ
      %w[แป้ง แปล๋น],          # ง < ล, tone ignored
      %w[เก็บ เกม],            # บ < ม, ไม้ไต่คู้ ignored
      %w[เกร็ง เกเร],          # 4th: ง < เ, ไม้ไต่คู้ ignored
      %w[ไส้ไก่ ไสยาสน์],       # 3rd: ก < ย, tone ignored
      %w[แหง่ แห่ง],           # tone position: ห (no mark) < ห่ (mark)
      %w[แหง้ แห่ง],           # tone position: no mark < mark, even with different tones
    ]
    pairs.each do |a, b|
      assert_equal [a, b], Thapthim.thai_sort([b, a]), "expected #{a} < #{b}"
    end
  end

  def test_ri_tone_only_ladder
    ladder = %w[เก็ง เก่ง เก้ง เก๋ง] # differ only by mark on ก: ็ ่ ้ ๋
    assert_equal ladder, Thapthim.thai_sort(ladder.reverse)
  end

  def test_karan_sorts_after_tone_marks
    # RI secondary order is ็ ่ ้ ๊ ๋ ์ — thanthakhat/การันต์ (์) is LAST, so a tone mark sorts before it.
    assert_equal ["แคร่", "แคร์"], Thapthim.thai_sort(["แคร์", "แคร่"]) # ่ (ek) before ์ (karan)
  end

  def test_nfc_normalizes_combining_mark_order
    # A below-vowel U+0E39 and a tone U+0E48 in the two possible orders render identically;
    # they must collate equal (NFC canonicalizes the combining-mark order before keying).
    canonical     = [0x0E04, 0x0E23, 0x0E39, 0x0E48].pack("U*") # kho ro sara-uu mai-ek
    non_canonical = [0x0E04, 0x0E23, 0x0E48, 0x0E39].pack("U*") # kho ro mai-ek sara-uu
    refute_equal canonical, non_canonical, "the two encodings must differ in bytes"
    assert_equal Thapthim.thai_sort_key(canonical), Thapthim.thai_sort_key(non_canonical)
  end

  def test_handles_empty_and_hardened_input
    assert_equal ["", "กา"], Thapthim.thai_sort(["กา", ""])
    # nil is hardened to "" (not raised), and an empty string sorts first.
    assert_equal Thapthim.thai_sort_key(""), Thapthim.thai_sort_key(nil)
  end

  # --- extended edge cases (expected orders derived from the Royal Institute rules) ---------------

  def test_full_tone_ladder
    # no mark < ไม้เอก < ไม้โท < ไม้ตรี < ไม้จัตวา, all on the same base letters.
    assert_equal ["กา", "ก่า", "ก้า", "ก๊า", "ก๋า"],
                 Thapthim.thai_sort(["ก๋า", "ก้า", "กา", "ก่า", "ก๊า"])
  end

  def test_no_vowel_then_mai_han_akat_then_sara_aa
    # กด (no written vowel) < กัด (ั) < กาด (า)
    assert_equal ["กด", "กัด", "กาด"], Thapthim.thai_sort(["กาด", "กด", "กัด"])
  end

  def test_multipart_leading_vowels_group_by_base_consonant
    assert_equal ["แกะ", "เปีย", "เสือ"], Thapthim.thai_sort(["เสือ", "แกะ", "เปีย"]) # ก < ป < ส
  end

  def test_leading_vowel_with_cluster_uses_second_consonant
    # เกม / เกรง / เกลา differ at the consonant after เก: ม < ร < ล
    assert_equal ["เกม", "เกรง", "เกลา"], Thapthim.thai_sort(["เกลา", "เกรง", "เกม"])
  end

  def test_rue_among_ro_and_lo_words
    assert_equal ["รัก", "ฤกษ์", "ลม"], Thapthim.thai_sort(["ลม", "ฤกษ์", "รัก"]) # ร < ฤ < ล
  end

  def test_rue_initial_words_order_by_next_letter
    assert_equal ["ฤกษ์", "ฤดู", "ฤทธิ์"], Thapthim.thai_sort(["ฤทธิ์", "ฤกษ์", "ฤดู"]) # ก < ด < ท
  end

  def test_rue_long_is_its_own_letter_after_rue
    # ฤๅ (and ฦๅ) sort as distinct letters: ร < ฤ(+letter) < ฤๅ < ล < ฦ(+letter) < ฦๅ < ว
    assert_equal ["รวย", "ฤกษ์", "ฤๅ", "ลม"], Thapthim.thai_sort(["ลม", "ฤๅ", "ฤกษ์", "รวย"])
    assert_equal ["ลม", "ฦๅ", "วัด"], Thapthim.thai_sort(["วัด", "ฦๅ", "ลม"])
  end

  def test_karan_word_is_prefix_of_longer_word
    # การันต์ (์) is secondary, so รักษ์ compares as [ร ั ก ษ] — a prefix of รักษา — and sorts first.
    assert_equal ["รักษ์", "รักษา"], Thapthim.thai_sort(["รักษา", "รักษ์"])
  end

  def test_full_vowel_sequence_on_same_consonant
    assert_equal ["กะ", "กา", "กิ", "กู", "เก", "โก"],
                 Thapthim.thai_sort(["กู", "กา", "กิ", "เก", "กะ", "โก"])
  end

  def test_bare_consonant_is_prefix
    assert_equal ["ก", "กา", "กาง"], Thapthim.thai_sort(["กา", "ก", "กาง"])
  end

  def test_mixed_scripts_thai_then_digits_then_latin
    assert_equal ["กา", "ไก่", "123", "apple"],
                 Thapthim.thai_sort(["apple", "กา", "123", "ไก่"])
  end

  def test_realistic_word_list
    input    = ["นก", "ปลา", "แมว", "หมา", "กบ", "ไก่"]
    expected = ["กบ", "ไก่", "นก", "ปลา", "แมว", "หมา"] # ก(กบ,ไก่) น ป ม(แมว) ห
    assert_equal expected, Thapthim.thai_sort(input)
  end

  def test_sort_key_reproduces_sort_over_larger_set
    words = %w[กา ไก่ ขา ไข่ เกา ป่า ปา กด กัด กาด เสือ แกะ ฤกษ์ ลม รัก นก หมา แมว]
    assert_equal Thapthim.thai_sort(words), words.sort_by { |w| Thapthim.thai_sort_key(w) }
  end

  def test_spaces_are_ignored_per_ri_convention
    assert_equal Thapthim.thai_sort_key("กรุงเทพ"), Thapthim.thai_sort_key("กรุง เทพ")
    assert_equal Thapthim.thai_sort_key("เป็นไทย"), Thapthim.thai_sort_key("เป็น  ไทย")
  end

  # Multi-syllable words need NO segmenter: collation is per-character, and the leading-vowel reorder
  # is applied to every เ/แ/โ/ใ/ไ in the string, wherever its syllable falls.

  def test_multisyllable_sort_without_segmenter
    input    = ["ประเทศ", "ประชา", "ปฏิบัติ", "ปกครอง"]
    expected = ["ปกครอง", "ปฏิบัติ", "ประชา", "ประเทศ"] # ก < ฏ < ร; then ช < ท
    assert_equal expected, Thapthim.thai_sort(input)
  end

  def test_multisyllable_leading_vowel_in_later_syllable
    # โรงงาน / โรงเรียน / โรงแรม — the second syllable's leading vowel (เ, แ) is reordered too.
    input    = ["โรงแรม", "โรงเรียน", "โรงงาน"]
    expected = ["โรงงาน", "โรงเรียน", "โรงแรม"] # after โรง-: ง < ร; then เ < แ
    assert_equal expected, Thapthim.thai_sort(input)
  end

  # Compound words (คำประสม) need no special handling — a compound is just a longer string, sorted
  # by its full (reordered) character sequence, whether written solid or with a space.

  def test_compound_words_rot
    assert_equal ["รถไฟ", "รถเมล์", "รถยนต์"],
                 Thapthim.thai_sort(["รถยนต์", "รถไฟ", "รถเมล์"]) # 3rd letter: ฟ < ม < ย
  end

  def test_compound_words_nam_shared_tone_and_sara_am_prefix
    assert_equal ["น้ำแข็ง", "น้ำตา", "น้ำมัน"],
                 Thapthim.thai_sort(["น้ำมัน", "น้ำแข็ง", "น้ำตา"]) # after น้ำ-: ข < ต < ม
  end

  def test_compound_written_with_space_collates_as_solid
    assert_equal Thapthim.thai_sort_key("รถไฟ"), Thapthim.thai_sort_key("รถ ไฟ")
  end

  # A sentence is just a (long) string: sorting a list of sentences orders them by first word, then
  # the next, and so on — character by character, spaces ignored.
  def test_sort_a_list_of_sentences
    input    = ["แมวนอนหลับ", "หมาเห่า", "แมวกินปลา", "ฉันรักแมว", "แมวดำ"]
    expected = ["ฉันรักแมว", "แมวกินปลา", "แมวดำ", "แมวนอนหลับ", "หมาเห่า"] # ฉ < แมว(ม) < ห; กิน<ดำ<นอน
    assert_equal expected, Thapthim.thai_sort(input)
  end
end
