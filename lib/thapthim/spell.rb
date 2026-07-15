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
end
