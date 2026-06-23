# tools/build_char_entropy.rb
#
# Offline builder for the character branching-entropy table used by the OOV-merge pass
# in segment_words (see ext/thapthim/src/lattice/decode.rs + entropy.rs).
#
# Branching entropy (Harris 1955; Jin & Tanaka-Ishii 2006): at a true word boundary the
# next character is uncertain (high Shannon entropy); inside a word it is nearly determined
# (low entropy). We build TWO directional tables from tnhc_train.json:
#
#   F  forward  right-branching : key = left context x,  H = entropy of the NEXT char after x
#   B  backward left-branching  : key = right context y, H = entropy of the PREV char before y
#
# Contexts run from length 1..K (the runtime backs off longest-first). Contexts observed fewer
# than MIN times are dropped: their entropy is unreliable (a context seen once has H=0, which
# would force a spurious merge) — the runtime backs off to a shorter, better-supported context.
#
# Output: ext/thapthim/assets/char_entropy.txt   (lines: "dir<TAB>context<TAB>entropy")
# Re-run this whenever K/MIN or the source corpus changes, then recompile the Rust extension
# (the table is embedded via include_str!). The decision THRESHOLD is NOT here — it is a runtime
# knob (THAPTHIM_BE_THRESHOLD), so tuning it needs no rebuild.
#
require "json"

K   = 4   # keep in sync with BE_MAX_CTX in lattice/mod.rs
MIN = 5

src = File.expand_path("../datasets/tnhc_train.json", __dir__)
out = File.expand_path("../ext/thapthim/assets/char_entropy.txt", __dir__)

sents = JSON.parse(File.read(src))

fwd_next = Hash.new { |h, k| h[k] = Hash.new(0) } # left_ctx  -> {next_char => count}
bwd_prev = Hash.new { |h, k| h[k] = Hash.new(0) } # right_ctx -> {prev_char => count}

sents.each do |tokens|
  chars = tokens.join.chars
  n = chars.size
  (0...n).each do |i|
    (1..K).each do |l|
      fwd_next[chars[(i - l)...i].join][chars[i]] += 1 if i - l >= 0
      bwd_prev[chars[i...(i + l)].join][chars[i - 1]] += 1 if i + l <= n && i - 1 >= 0
    end
  end
end

def shannon(dist)
  total = dist.values.sum.to_f
  -dist.values.sum { |c| p = c / total; p * Math.log2(p) }
end

File.open(out, "w") do |f|
  [["F", fwd_next], ["B", bwd_prev]].each do |dir, table|
    table.each do |ctx, dist|
      next if dist.values.sum < MIN
      f.puts "#{dir}\t#{ctx}\t#{format("%.5f", shannon(dist))}"
    end
  end
end

lines = File.foreach(out).count
puts "wrote #{out}"
puts "  #{lines} contexts  (#{(File.size(out) / 1_000_000.0).round(1)} MB), K=#{K}, MIN=#{MIN}"
