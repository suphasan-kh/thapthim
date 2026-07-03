# lib/thapthim/segment.rb
require 'fiddle'

module Thapthim
  # Reopen the bridge created in segment_tcc.rb (the library is already dlloaded there) and
  # attach the word/syllable segmentation entry points.
  module NativeBridge
    extern 'void* thapthim_segment(void*, void*, int)'
    extern 'void* thapthim_segment_syllables(void*, void*, int)'
    extern 'void thapthim_free_u64_array(void*, int)'
  end

  # Calls a native segmentation function that returns a packed u64 token stream and decodes it
  # back into substrings of the original text. Each token packs [ Start:32 | Length:24 | Tier:8 ]
  # as byte offsets, so we slice on bytes (TCC boundaries are always valid UTF-8 boundaries).
  # Whitespace-only tokens are dropped on the Rust side unless +keep_whitespace+ is true.
  def self.decode_tokens(input_text, fn_name, keep_whitespace)
    text = sanitize_input(input_text)
    return [] if text.empty?

    text_pointer = Fiddle::Pointer.to_ptr(text)
    # RUBY_FREE so the out-param buffer is reclaimed at GC — malloc without a free
    # function is never freed and leaks 4 bytes per call.
    size_buffer = Fiddle::Pointer.malloc(Fiddle::SIZEOF_INT, Fiddle::RUBY_FREE)

    raw_address = NativeBridge[fn_name].call(text_pointer.to_i, size_buffer.to_i, keep_whitespace ? 1 : 0)
    return [] if raw_address.nil? || raw_address == 0

    raw_ptr = Fiddle::Pointer.new(raw_address)
    count = size_buffer[0, Fiddle::SIZEOF_INT].unpack1('i')
    begin
      packed = count.zero? ? [] : raw_ptr[0, count * 8].unpack('Q*')
    ensure
      # The count is what free needs to reconstruct the slice, so it is read before this
      # block; the ensure keeps the Rust buffer from leaking if the copy-out raises.
      NativeBridge['thapthim_free_u64_array'].call(raw_ptr.to_i, count)
    end

    packed.map do |token|
      start = token >> 32
      length = (token >> 8) & 0xFFFFFF
      text.byteslice(start, length)
    end
  end
  private_class_method :decode_tokens

  # Word-level segmentation. Returns an array of word strings.
  #
  # Pass +normalize: true+ to first run the input through +std_normalize+ (collapse repeated
  # vowels, strip zero-width/dangling marks, reorder vowels) for noisy/OCR-derived text. Note
  # the returned tokens are then substrings of the *normalized* text, not the original.
  #
  # Whitespace-only tokens are omitted by default. Pass +keep_whitespace: true+ to include
  # them, which makes the returned tokens a lossless tiling of the input
  # (<tt>tokens.join == text</tt>).
  def self.word_segment(input_text, normalize: false, keep_whitespace: false)
    text = normalize ? std_normalize(input_text.to_s) : input_text
    decode_tokens(text, 'thapthim_segment', keep_whitespace)
  end

  # Syllable-level segmentation. Returns an array of syllable strings; their boundaries are a
  # superset of the word boundaries returned by +word_segment+. See +word_segment+ for
  # +normalize:+ and +keep_whitespace:+.
  def self.syllable_segment(input_text, normalize: false, keep_whitespace: false)
    text = normalize ? std_normalize(input_text.to_s) : input_text
    decode_tokens(text, 'thapthim_segment_syllables', keep_whitespace)
  end
end
