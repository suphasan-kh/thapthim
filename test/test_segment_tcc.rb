# test/test_segment_tcc.rb
require "minitest/autorun"
require_relative "../lib/thapthim"

class TestSegmentTcc < Minitest::Test
  def setup
    @sentence = "ทนายปีศาจ เนี่ยเริ่มต้นจากแซม"
  end

  def test_that_it_segments_thai_character_clusters_correctly
    expected_chunks = ["ท", "นา", "ย", "ปี", "ศา", "จ", " ", "เนี่ย", "เริ่ม", "ต้", "น", "จา", "ก", "แซ", "ม"]
    assert_equal expected_chunks, Thapthim.tcc_segment(@sentence)
  end

  def test_that_it_returns_byte_based_positions
    # Verifies the byte-accurate offsets coming back from Rust
    positions = Thapthim.tcc_positions(@sentence)
    assert_equal 0, positions.first
    assert_equal 43, positions[8] # 'เริ่ม' byte boundary tracking
  end

  def test_handling_empty_or_nil_inputs
    assert_equal [], Thapthim.tcc_segment("")
    assert_equal [], Thapthim.tcc_segment(nil)
    assert_equal [0], Thapthim.tcc_positions("")
  end
end