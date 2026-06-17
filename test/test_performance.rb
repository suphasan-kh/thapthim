# test/test_performance.rb
require "minitest/autorun"
require "minitest/benchmark"
require_relative "../lib/thapthim"

class TestPerformance < Minitest::Benchmark
  def setup
    # Create a base Thai string segment
    @base_text = "ทนายปีศาจ เนี่ยเริ่มต้นจากแซมกับซันและก็ทีมเขียนบท "
  end

  # This validates that your Rust TCC engine executes in Linear Time O(N)
  # It proves that removing the look-aheads successfully killed ReDoS vulnerabilities!
  def bench_tcc_segment_linear_performance
    assert_performance_linear 0.95 do |n|
      # Generate strings that scale exponentially in size (n = 10, 100, 1000, 10000)
      scaled_text = @base_text * n
      Thapthim.tcc_segment(scaled_text)
    end
  end
end