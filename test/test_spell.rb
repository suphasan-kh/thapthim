# test/test_spell.rb
# Locks down the word-level spelling API (spell / suggest / correct) — the Norvig noisy channel over
# the thai_words dictionary + unigram LM. Covers detection, candidate ranking, correction, the
# valid-word no-op, and the input-hardening contract inherited from sanitize_input.
require "minitest/autorun"
require_relative "../lib/thapthim"

class TestSpell < Minitest::Test
  # --- spell: detection ------------------------------------------------------------

  def test_spell_true_for_valid_word
    assert Thapthim.spell("กิน")
    assert Thapthim.spell("ประเทศ")
  end

  def test_spell_false_for_misspelling
    refute Thapthim.spell("กนิ")   # transposed
    refute Thapthim.spell("ประเทด") # ศ→ด
  end

  def test_spell_false_for_empty_or_nil
    refute Thapthim.spell("")
    refute Thapthim.spell(nil)
  end

  # --- suggest: ranked candidates --------------------------------------------------

  def test_suggest_ranks_intended_word_first
    assert_equal "เรียน", Thapthim.suggest("เรยน").first
    assert_equal "ประเทศ", Thapthim.suggest("ประเทด").first
  end

  def test_suggest_returns_array_and_includes_intended
    cands = Thapthim.suggest("อนุญาติ")
    assert_kind_of Array, cands
    assert_includes cands, "อนุญาต"
  end

  def test_suggest_empty_for_blank_input
    assert_equal [], Thapthim.suggest("")
    assert_equal [], Thapthim.suggest(nil)
  end

  # --- correct: best pick / no-op --------------------------------------------------

  def test_correct_fixes_typo
    assert_equal "ผลไม้", Thapthim.correct("ผลไม")
    assert_equal "อนุญาต", Thapthim.correct("อนุญาติ")
  end

  def test_correct_leaves_valid_word_unchanged
    assert_equal "กิน", Thapthim.correct("กิน")
    assert_equal "ประเทศ", Thapthim.correct("ประเทศ")
  end

  def test_correct_empty_input
    assert_equal "", Thapthim.correct("")
  end

  # --- input hardening -------------------------------------------------------------

  def test_handles_non_string_input
    # coerced via to_s; a number has no Thai candidates, so it comes back unchanged/empty-safe
    assert_kind_of String, Thapthim.correct(12345)
    assert_kind_of Array, Thapthim.suggest(12345)
  end

  # --- correct_sent: context-aware correction --------------------------------------

  def test_correct_sent_fixes_typo_in_context
    # กนิ -> กิน (in the LM; ชอบ→กิน bigram); the context picks it.
    assert_equal ["ผม", "ชอบ", "กิน", "ข้าว"], Thapthim.correct_sent(["ผม", "ชอบ", "กนิ", "ข้าว"])
    assert_equal ["เด็ก", "ไป", "โรงเรียน"], Thapthim.correct_sent(["เด็ก", "ไป", "โรงเรยน"])
  end

  def test_correct_sent_leaves_valid_text_unchanged
    assert_equal ["ผม", "ชอบ", "กิน", "ข้าว"], Thapthim.correct_sent(["ผม", "ชอบ", "กิน", "ข้าว"])
  end

  def test_correct_sent_uses_aligned_bigram_for_context
    # ความสามาด is OOV; ความสะอาด and ความสามารถ are both edit-distance 2. The aligned bigram
    # (มี→ความสามารถ) resolves it in context — the LST20 bigram can't (it lacks ความสามารถ).
    assert_equal ["เขา", "มี", "ความสามารถ"], Thapthim.correct_sent(["เขา", "มี", "ความสามาด"])
  end

  def test_correct_sent_string_returns_string
    result = Thapthim.correct_sent("ผมชอบกินข้าว")
    assert_kind_of String, result
  end

  def test_correct_sent_empty
    assert_equal [], Thapthim.correct_sent([])
  end
end
