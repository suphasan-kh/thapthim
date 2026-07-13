# tools/orchid_to_jsonl.rb
#
# Convert the ORCHID97 POS-tagged corpus (datasets/orchid97.txt) into the JSONL gold-token
# format that test/eval_segment.rb consumes: one JSON array of gold word strings per line,
# tokens tiling the sentence exactly (whitespace is its own token).
#
# ORCHID layout (per sentence):
#   #P<n>                     paragraph marker            (skipped)
#   #<n>                      sentence number             (skipped)
#   <raw untokenized text>//  the surface, ends in //     (skipped — empty tag)
#   word/TAG                  one tokenized word per line  (collected)
#   ...
#   //                        sentence terminator          (flush)
#   %...                      bibliographic metadata       (skipped)
#
# A token line is `surface/TAG`; the surface may itself contain a space ("ที่ 1/DONM") and even
# a slash via the <slash> escape, so we split on the LAST slash and require the suffix to be a
# bare uppercase tag. Punctuation and spaces are ORCHID <name> escapes, decoded below; unknown
# escapes (rare linguistics clause labels like <b>, Miller index <100>) appear literally in the
# raw text too, so they are left as-is.
#
# Sentences are deterministically shuffled (seed=42) — matching the shuffled BEST/VISTEC splits —
# so a first-N cap in the benchmark harness is a representative sample rather than the first few
# conference proceedings (ORCHID is stored in document order). Micro-averaged full-set F1 is
# order-independent, so shuffling does not change the headline number.
#
# Usage: ruby tools/orchid_to_jsonl.rb datasets/orchid97.txt datasets/orchid_test.jsonl
require "json"

ESCAPES = {
  "<space>" => " ", "<left_parenthesis>" => "(", "<right_parenthesis>" => ")",
  "<full_stop>" => ".", "<quotation>" => "\"", "<comma>" => ",", "<minus>" => "-",
  "<slash>" => "/", "<equal>" => "=", "<colon>" => ":", "<asterisk>" => "*",
  "<plus>" => "+", "<greater_than>" => ">", "<less_than>" => "<", "<semi_colon>" => ";",
  "<apostrophe>" => "'", "<dollar>" => "$", "<number>" => "#", "<at_mark>" => "@",
  "<ampersand>" => "&", "<question_mark>" => "?", "<circumflex_accent>" => "^",
  "<exclamation>" => "!", "<left_curly_bracket>" => "{"
}.freeze
ESCAPE_RE = /#{ESCAPES.keys.map { |k| Regexp.escape(k) }.join("|")}/

TAG_RE = /\A[A-Z]+\z/ # ORCHID POS tags are bare uppercase letters (NCMN, VACT, PUNC, ...)

def decode(surface)
  surface.gsub(ESCAPE_RE) { |m| ESCAPES[m] }
end

# Parse one ORCHID token line into its decoded surface, or nil if the line isn't a token.
def token_surface(line)
  idx = line.rindex("/")
  return nil unless idx && idx.positive?
  tag = line[(idx + 1)..]
  return nil unless tag&.match?(TAG_RE)
  decode(line[0...idx])
end

infile  = ARGV[0] || "datasets/orchid97.txt"
outfile = ARGV[1] || "datasets/orchid_test.jsonl"

all = []
current = []
flush = lambda do
  all << current unless current.empty?
  current = []
end

File.foreach(infile) do |line|
  line = line.chomp
  surface = line.empty? ? nil : token_surface(line)
  if surface.nil?
    flush.call # any non-token line (marker, raw text, //, metadata) ends the current sentence
  else
    current << surface
  end
end
flush.call

all.shuffle!(random: Random.new(42))
File.open(outfile, "w") { |out| all.each { |s| out.puts JSON.generate(s) } }

warn "wrote #{all.length} sentences, #{all.sum(&:length)} tokens -> #{outfile}"
