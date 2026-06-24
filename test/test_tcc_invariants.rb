# test/test_tcc_invariants.rb
# Structural invariants for the TCC layer (Thapthim.tcc_positions / Thapthim.tcc_segment).
# Basic expected-output cases live in test_segment_tcc.rb; this file pins the properties that must
# hold for *any* input, where boundary/offset bugs in the FFI marshalling would surface:
#   - positions are CHARACTER offsets (not bytes): start at 0, end at text.length, strictly increasing
#   - tcc_segment reconstructs the input losslessly and agrees with tcc_positions slicing
#   - multibyte / astral text does not desynchronize the offsets
require "minitest/autorun"
require_relative "../lib/thapthim"

class TestTccInvariants < Minitest::Test
  NUL = 0.chr

  CORPUS = [
    "ฉันกินข้าว",
    "ทนายปีศาจ เนี่ยเริ่มต้นจากแซม",
    "iPhone15ราคาดี😀",
    "ก\nข\tค",
    "👨‍👩‍👧 ครอบครัว",
    "ปี ๒๕๖๙ เวลา ๑๔.๓๐ น.",
    "ก่่่่่ข",
  ].freeze

  # --- tcc_positions: a valid, character-based boundary list ----------------------

  def test_positions_start_at_zero
    CORPUS.each { |s| assert_equal 0, Thapthim.tcc_positions(s).first, "for #{s.inspect}" }
  end

  def test_positions_end_at_character_length
    CORPUS.each do |s|
      assert_equal s.length, Thapthim.tcc_positions(s).last,
                   "last boundary must be the CHARACTER length for #{s.inspect}"
    end
  end

  def test_positions_are_strictly_increasing
    CORPUS.each do |s|
      pos = Thapthim.tcc_positions(s)
      assert(pos.each_cons(2).all? { |a, b| b > a },
             "positions must be strictly increasing for #{s.inspect}: #{pos.inspect}")
    end
  end

  def test_positions_are_character_offsets_not_bytes
    # "😀" is 1 character but 4 UTF-8 bytes; a byte-based boundary would overshoot text.length.
    pos = Thapthim.tcc_positions("ก😀ข")
    assert_equal 3, pos.last, "char length of \"ก😀ข\" is 3"
    assert pos.all? { |p| p <= 3 }, "no boundary may exceed the character length: #{pos.inspect}"
  end

  # --- tcc_segment: lossless and consistent with the positions --------------------

  def test_tcc_segment_is_lossless
    CORPUS.each do |s|
      assert_equal s, Thapthim.tcc_segment(s).join,
                   "tcc_segment must reconstruct the input for #{s.inspect}"
    end
  end

  def test_tcc_segment_matches_positions
    CORPUS.each do |s|
      from_positions = Thapthim.tcc_positions(s).each_cons(2).map { |a, b| s[a...b] }
      assert_equal from_positions, Thapthim.tcc_segment(s),
                   "tcc_segment must equal positions-based slicing for #{s.inspect}"
    end
  end

  def test_tcc_tokens_are_non_empty_and_valid_utf8
    CORPUS.each do |s|
      Thapthim.tcc_segment(s).each do |t|
        refute_empty t, "no empty TCC token for #{s.inspect}"
        assert t.valid_encoding?, "TCC token must be valid UTF-8 for #{s.inspect}: #{t.inspect}"
        refute_includes t, NUL
      end
    end
  end

  # --- empty / nil edge cases (positions has its own [0] convention) ---------------

  def test_empty_and_nil_positions
    assert_equal [0], Thapthim.tcc_positions("")
    assert_equal [0], Thapthim.tcc_positions(nil)
  end

  def test_determinism
    CORPUS.each do |s|
      assert_equal Thapthim.tcc_positions(s), Thapthim.tcc_positions(s)
      assert_equal Thapthim.tcc_segment(s), Thapthim.tcc_segment(s)
    end
  end
end
