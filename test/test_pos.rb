# test/test_pos.rb
# Behavioural tests for the POS-tagging API (Thapthim.pos_tag). Pins down the *contract*, not
# accuracy (that lives in eval_pos.rb):
#   - one [surface, tag] pair per input token, tags drawn from POS_TAGS
#   - array input tags gold tokens verbatim; string input cascades through word_segment
#   - the string cascade's surfaces reassemble the (sanitized) input (losslessness carries through)
#   - empty input -> []
#   - determinism : identical input -> identical output
#
# Uses the embedded model (assets/pos_hmm.bin), so no env var is needed; `THAPTHIM_POS_MODEL` can
# still override it for experiments.
require "minitest/autorun"
require_relative "../lib/thapthim"

class TestPos < Minitest::Test
  def test_array_input_tags_each_token
    tokens = %w[ผม รัก ประเทศ ไทย]
    result = Thapthim.pos_tag(tokens)

    assert_equal tokens.length, result.length
    assert_equal tokens, result.map(&:first) # surfaces preserved verbatim, in order
    result.each do |surface, tag|
      assert_kind_of String, surface
      assert_includes Thapthim::POS_TAGS, tag, "#{tag.inspect} is not a known LST20 tag"
    end
  end

  def test_string_input_cascades_and_is_lossless
    text = "ฉันรักภาษาไทย"
    result = Thapthim.pos_tag(text)

    refute_empty result
    assert_equal text, result.map(&:first).join, "cascade surfaces should reassemble the input"
    result.each { |_, tag| assert_includes Thapthim::POS_TAGS, tag }
  end

  def test_digit_shape_rule_tags_numbers
    # An all-digit token is hard-routed to NU by the OOV shape rule even when unseen.
    result = Thapthim.pos_tag(["1234567890"])
    assert_equal "NU", result.first.last
  end

  def test_foreign_shape_rule_tags_nn
    # Foreign-script OOV tokens (Latin and CJK) are tagged NN — the LST20 convention — even inside a
    # punctuation context that would otherwise drag them to NU (the `( ... )` → number bias).
    assert_equal "NN", Thapthim.pos_tag(["Microsoft"]).first.last
    tagged = Thapthim.pos_tag(["(", "人間宣言", ")"]).to_h
    assert_equal "NN", tagged["人間宣言"]
  end

  def test_karan_shape_rule_tags_nn
    # A karan-marked (◌์) OOV token — transliterations/loanwords in Thai script that is_foreign can't
    # see — is tagged NN, even in a punctuation context that would otherwise pull it to NU.
    # อินโนเซ้นท์ ("Innocent") is a real OOV translit name that regressed to NU before this rule.
    tagged = Thapthim.pos_tag(["(", "อินโนเซ้นท์", ")"]).to_h
    assert_equal "NN", tagged["อินโนเซ้นท์"]
  end

  def test_noun_morphology_rule_tags_nn
    # Productive Thai noun affixes (การ/ความ/ผู้ prefix, ศาสตร์ suffix) force NN for OOV words even in a
    # NU-pulling context. ฟรุ้งฟริ้ง is slang, so the compounds are OOV.
    h = ->(w) { Thapthim.pos_tag(["(", w, ")"]).to_h[w] }
    assert_equal "NN", h.call("ความฟรุ้งฟริ้ง") # ความ nominalizer prefix
    assert_equal "NN", h.call("ผู้ฟรุ้งฟริ้ง")   # ผู้ agentive prefix
  end

  def test_empty_input
    assert_equal [], Thapthim.pos_tag("")
    assert_equal [], Thapthim.pos_tag([])
  end

  def test_deterministic
    tokens = %w[รัฐบาล แถลง นโยบาย]
    assert_equal Thapthim.pos_tag(tokens), Thapthim.pos_tag(tokens)
  end
end
