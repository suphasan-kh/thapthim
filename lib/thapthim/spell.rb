# lib/thapthim/spell.rb
require 'fiddle'

module Thapthim
  # Reopen the bridge created in segment_tcc.rb (the library is already dlloaded there) and attach
  # the word-level spelling entry points. Words cross the C boundary as NUL-terminated UTF-8 strings;
  # suggest() returns its candidates newline-joined in one buffer (freed with thapthim_free_string).
  module NativeBridge
    extern 'int thapthim_spell(void*)'
    extern 'char* thapthim_suggest(void*)'
    extern 'char* thapthim_correct(void*)'
    extern 'char* thapthim_correct_sent(void*, void*, int, void*)'
  end

  # True if +word+ is a correctly-spelled Thai word (a dictionary entry, after normalization).
  # Empty/blank input is not a word.
  def self.spell(word)
    text = sanitize_input(word)
    return false if text.empty?

    NativeBridge['thapthim_spell'].call(Fiddle::Pointer.to_ptr(text).to_i) == 1
  end

  # Ranked spelling suggestions for +word+ (best first), or +[]+ if none are within the edit bound.
  # The candidates are dictionary words scored by the noisy channel (unigram LM × edit distance).
  def self.suggest(word)
    text = sanitize_input(word)
    return [] if text.empty?

    address = NativeBridge['thapthim_suggest'].call(Fiddle::Pointer.to_ptr(text).to_i)
    return [] if address.nil? || address == 0

    begin
      joined = Fiddle::Pointer.new(address).to_s.force_encoding(Encoding::UTF_8)
    ensure
      NativeBridge['thapthim_free_string'].call(address)
    end
    joined.empty? ? [] : joined.split("\n")
  end

  # Best-effort correction of a single +word+: a valid word is returned unchanged (normalized), an
  # unknown word becomes its top-ranked candidate, or is left unchanged if nothing is close enough.
  def self.correct(word)
    text = sanitize_input(word)
    return text if text.empty?

    address = NativeBridge['thapthim_correct'].call(Fiddle::Pointer.to_ptr(text).to_i)
    return text if address.nil? || address == 0

    begin
      Fiddle::Pointer.new(address).to_s.force_encoding(Encoding::UTF_8)
    ensure
      NativeBridge['thapthim_free_string'].call(address)
    end
  end

  # Context-aware correction of a whole sentence, using the bigram LM to pick each correction in
  # context (a valid word/name that fits its neighbours is kept; an unknown word is corrected toward
  # the candidate the context favours). Accepts a String (segmented first) or an already-tokenized
  # Array. Mirrors the input: a String returns the corrected String, an Array returns corrected
  # tokens. Cascade for the String case (segment → correct); joint segmentation+correction is future.
  def self.correct_sent(input)
    if input.is_a?(Array)
      correct_tokens(input.map { |t| sanitize_input(t) })
    else
      correct_tokens(word_segment(sanitize_input(input), keep_whitespace: true)).join
    end
  end

  # Run the native sentence corrector over an array of tokens; returns the corrected token array.
  def self.correct_tokens(tokens)
    return [] if tokens.empty?

    concat = tokens.join
    lengths = tokens.map(&:bytesize).pack('l*') # 32-bit signed, matches the C `int` lengths array
    out_lengths = Fiddle::Pointer.malloc(Fiddle::SIZEOF_INT * tokens.length, Fiddle::RUBY_FREE)

    address = NativeBridge['thapthim_correct_sent'].call(
      Fiddle::Pointer.to_ptr(concat).to_i, Fiddle::Pointer.to_ptr(lengths).to_i, tokens.length, out_lengths.to_i
    )
    return tokens if address.nil? || address == 0 # native fallback: leave unchanged

    begin
      joined = Fiddle::Pointer.new(address).to_s.dup.force_encoding(Encoding::BINARY)
    ensure
      NativeBridge['thapthim_free_string'].call(address)
    end

    lens = out_lengths[0, Fiddle::SIZEOF_INT * tokens.length].unpack('l*')
    corrected = []
    offset = 0
    lens.each do |len|
      corrected << joined.byteslice(offset, len).to_s.force_encoding(Encoding::UTF_8)
      offset += len
    end
    corrected
  end
  private_class_method :correct_tokens
end
