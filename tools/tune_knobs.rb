# tools/tune_knobs.rb
#
# Grid-search driver for Thapthim's runtime knobs. Sweeps one knob across a list of
# values, runs the word-segmentation eval (test/eval_segment.rb) once per value, and
# prints a ranked F1 table plus the argmax. This is how the shipped defaults
# (THAPTHIM_BE_THRESHOLD=1.0, THAPTHIM_OOV_PENALTY=2.0) are meant to be re-derived.
#
# Each value runs in its own subprocess on purpose: the engine reads the knob ONCE at
# bootstrap (see ext/thapthim/src/lattice/mod.rs), so a value cannot change within a
# single process. No rebuild is needed — the knobs are environment-tunable.
#
# Usage:
#   ruby tools/tune_knobs.rb KNOB VAL [VAL ...] [options]
#
#   KNOB                  one of: be_threshold | be_max_tcc | oov_penalty
#   VAL ...               the grid of values to try (e.g. 1.5 2.0 2.5)
#
# Options:
#   --corpus NAME[,NAME]  corpus/corpora to evaluate (short name or JSON path).
#                         Default: tnhc. RANK is by the FIRST corpus listed.
#   --metric incl|excl    which F1 to optimize (default: excl = excluding spaces).
#   --limit N             cap sentences per corpus (passes THAPTHIM_EVAL_LIMIT).
#
# Examples:
#   ruby tools/tune_knobs.rb oov_penalty 1.0 1.5 2.0 2.5 3.0 --corpus tnhc_train
#   ruby tools/tune_knobs.rb be_threshold 0.5 0.75 1.0 1.25 1.5 --corpus tnhc_train,best --limit 3000
#
# IMPORTANT — tune on a HELD-OUT split, not the test sets. The shipped defaults were
# tuned on tnhc_train precisely so the test corpora (tnhc/lst20/best/vistec) stay an
# honest measurement. Sweeping directly on a test set overfits it; the driver warns
# when you do.

require "open3"

KNOB_ENV = {
  "be_threshold" => "THAPTHIM_BE_THRESHOLD",
  "be_max_tcc"   => "THAPTHIM_BE_MAX_TCC",
  "oov_penalty"  => "THAPTHIM_OOV_PENALTY",
}.freeze

# Test corpora known to eval_segment.rb — sweeping on these overfits, so we warn.
TEST_CORPORA = %w[tnhc lst20 best vistec].freeze

EVAL = File.expand_path("../test/eval_segment.rb", __dir__)

def die(msg)
  warn "tune_knobs: #{msg}"
  exit 1
end

# The name eval_segment.rb reports a corpus under: its short name if known, else the
# JSON file's basename. Used to match parsed F1 back to the requested corpus.
def report_name(arg)
  TEST_CORPORA.include?(arg) ? arg : File.basename(arg, ".json")
end

# ---- argument parsing -------------------------------------------------------

argv = ARGV.dup
knob = argv.shift
die "missing KNOB (one of: #{KNOB_ENV.keys.join(", ")})" if knob.nil?
env_var = KNOB_ENV[knob] or die "unknown knob '#{knob}' (one of: #{KNOB_ENV.keys.join(", ")})"

corpora = ["tnhc"]
metric  = :excl
limit   = nil
values  = []

until argv.empty?
  arg = argv.shift
  case arg
  when "--corpus" then corpora = (argv.shift or die("--corpus needs a value")).split(",")
  when "--metric"
    m = argv.shift
    die "--metric must be incl or excl" unless %w[incl excl].include?(m)
    metric = m.to_sym
  when "--limit"  then limit = (argv.shift or die("--limit needs a value"))
  else values << arg
  end
end

die "no values to sweep (give a grid, e.g. 1.0 1.5 2.0)" if values.empty?

overfit = corpora & TEST_CORPORA
unless overfit.empty?
  warn "tune_knobs: WARNING — #{overfit.join(", ")} #{overfit.size == 1 ? "is a" : "are"} test " \
       "corpus; tuning on it overfits. Prefer a held-out split (e.g. tnhc_train)."
end

# ---- sweep ------------------------------------------------------------------

# Parse eval_segment's report into { corpus_name => { incl:, excl: } }.
def parse_eval(out)
  results = {}
  current = nil
  out.each_line(chomp: true) do |line|
    if line =~ /\A\S.*\z/ && !line.start_with?("Thapthim", "metric:")
      current = line.strip
      results[current] = {}
    elsif current && line =~ /incl\. spaces.*F1=([\d.]+)/
      results[current][:incl] = Regexp.last_match(1).to_f
    elsif current && line =~ /excl\. spaces.*F1=([\d.]+)/
      results[current][:excl] = Regexp.last_match(1).to_f
    end
  end
  results
end

names   = corpora.map { |c| report_name(c) }
primary = names.first
puts "Sweeping #{env_var} over #{values.join(", ")}"
puts "corpora: #{names.join(", ")}  |  rank by: #{primary} F1 (#{metric}. spaces)" \
     "#{limit ? "  |  limit=#{limit}" : ""}"
puts

rows = values.map do |val|
  env = { env_var => val.to_s }
  env["THAPTHIM_EVAL_LIMIT"] = limit if limit
  out, status = Open3.capture2(env, RbConfig.ruby, EVAL, *corpora)
  die "eval failed for #{env_var}=#{val} (exit #{status.exitstatus})\n#{out}" unless status.success?
  parsed = parse_eval(out)
  die "could not parse eval output for #{env_var}=#{val}:\n#{out}" if parsed[primary].nil?
  { value: val, f1: parsed }
end

# ---- ranked table -----------------------------------------------------------

best = rows.max_by { |r| r[:f1].dig(primary, metric) || -1.0 }

label_w = [env_var.length, values.map(&:length).max].max
col_w   = 9
header = "#{env_var.ljust(label_w)}  " + names.map { |c| c.center(col_w) }.join("  ") + "   "
puts header
puts "-" * header.length

rows.each do |r|
  cells = names.map do |c|
    f = r[:f1].dig(c, metric)
    f ? format("%.4f", f).center(col_w) : "  --  ".center(col_w)
  end
  marker = r.equal?(best) ? " <= best" : ""
  puts "#{r[:value].to_s.ljust(label_w)}  #{cells.join("  ")}#{marker}"
end

puts
bf = best[:f1].dig(primary, metric)
puts "Best: #{env_var}=#{best[:value]}  (#{primary} F1 #{metric}.spaces = #{format("%.4f", bf)})"
puts "Apply at runtime with:  #{env_var}=#{best[:value]} ruby your_script.rb"
