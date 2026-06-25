# test/test_ffi_safety.rb
# The Ruby<->Rust boundary hand-marshals raw pointers: each call mallocs a buffer in Rust, Ruby
# copies out the packed token stream, then calls back into Rust to free it. Bugs in that dance
# (use-after-free, double-free, leaked buffers, off-by-one on the size word, GVL/threading hazards)
# tend to manifest only under repetition, scale, or concurrency rather than on a single happy-path
# call. These tests hammer the boundary to flush such errors out.
require "minitest/autorun"
require_relative "../lib/thapthim"

class TestFfiSafety < Minitest::Test
  # --- many alloc/free cycles must not corrupt state or drift results --------------

  def test_repeated_calls_stay_correct
    expected = Thapthim.word_segment("ฉันกินข้าวที่ร้านอาหารไทย")
    2_000.times do |i|
      # interleave the four entry points so their buffers churn together
      Thapthim.tcc_positions("ก")
      Thapthim.syllable_segment("ฉัน")
      Thapthim.std_normalize("  ก  ข  ")
      assert_equal expected, Thapthim.word_segment("ฉันกินข้าวที่ร้านอาหารไทย"),
                   "result drifted on iteration #{i} (memory corruption / stale buffer?)"
    end
  end

  # --- a single large input round-trips with the right token count ----------------

  def test_large_input_token_count_and_losslessness
    unit = "ฉันรักภาษาไทย"
    big = unit * 5_000
    tokens = Thapthim.word_segment(big)
    assert_equal 20_000, tokens.length, "expected 4 words x 5000 repetitions"
    assert_equal big, tokens.join, "large input must remain lossless"
  end

  # --- many distinct inputs in a row (no cross-call contamination) -----------------

  def test_distinct_inputs_do_not_bleed_into_each_other
    inputs = (1..500).map { |n| "คำที่#{n} " }
    inputs.each do |s|
      assert_equal s, Thapthim.word_segment(s).join,
                   "input #{s.inspect} did not round-trip — possible buffer reuse bug"
    end
  end

  # --- concurrency: the read-only engine must be safe across threads ---------------

  def test_concurrent_segmentation_is_correct
    sentence = "ทนายปีศาจ เนี่ยเริ่มต้นจากแซมกับซันและก็ทีมเขียนบท"
    expected = Thapthim.word_segment(sentence)

    errors = Queue.new
    threads = 8.times.map do
      Thread.new do
        300.times do
          got = Thapthim.word_segment(sentence)
          errors << got unless got == expected
        end
      rescue => e
        errors << e
      end
    end
    threads.each(&:join)

    assert_empty drain(errors),
                 "concurrent segmentation produced wrong or crashing results"
  end

  def test_concurrent_mixed_endpoints_do_not_crash
    threads = 6.times.map do |i|
      Thread.new do
        150.times do
          case i % 4
          when 0 then Thapthim.word_segment("ฉันกินข้าว")
          when 1 then Thapthim.syllable_segment("ฉันกินข้าว")
          when 2 then Thapthim.tcc_segment("ฉันกินข้าว")
          else        Thapthim.std_normalize("  ก  ข  ")
          end
        end
        :ok
      end
    end
    assert_equal [:ok] * 6, threads.map(&:value),
                 "every mixed concurrent endpoint thread must complete without raising"
  end

  # --- empty/zero-token paths exercise the count==0 free branch repeatedly ---------

  def test_empty_token_path_is_stable
    1_000.times do
      assert_equal [], Thapthim.word_segment("")
      assert_equal [], Thapthim.word_segment(0.chr * 3) # all-NUL -> sanitizes to empty
    end
  end

  private

  def drain(queue)
    out = []
    out << queue.pop until queue.empty?
    out
  end
end
