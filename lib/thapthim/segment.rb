# lib/thapthim/segment.rb
require 'fiddle'

module Thapthim
  # Reopen the bridge created in segment_tcc.rb (the library is already dlloaded there) and
  # attach the word/syllable segmentation entry points.
  module NativeBridge
    extern 'void* thapthim_segment(void*, void*)'
    extern 'void* thapthim_segment_syllables(void*, void*)'
    extern 'void thapthim_free_u64_array(void*, int)'
  end

  # Calls a native segmentation function that returns a packed u64 token stream and decodes it
  # back into substrings of the original text. Each token packs [ Start:32 | Length:24 | Tier:8 ]
  # as byte offsets, so we slice on bytes (TCC boundaries are always valid UTF-8 boundaries).
  def self.decode_tokens(input_text, fn_name)
    return [] if input_text.nil? || input_text.empty?

    text_pointer = Fiddle::Pointer.to_ptr(input_text.to_s)
    size_buffer = Fiddle::Pointer.malloc(Fiddle::SIZEOF_INT)

    raw_address = NativeBridge[fn_name].call(text_pointer.to_i, size_buffer.to_i)
    return [] if raw_address.nil? || raw_address == 0

    raw_ptr = Fiddle::Pointer.new(raw_address)
    count = size_buffer[0, Fiddle::SIZEOF_INT].unpack1('i')
    packed = count.zero? ? [] : raw_ptr[0, count * 8].unpack('Q*')

    NativeBridge['thapthim_free_u64_array'].call(raw_ptr.to_i, count)

    packed.map do |token|
      start = token >> 32
      length = (token >> 8) & 0xFFFFFF
      input_text.byteslice(start, length)
    end
  end
  private_class_method :decode_tokens

  # Word-level segmentation. Returns an array of word strings.
  #
  # Pass +normalize: true+ to first run the input through +std_normalize+ (collapse repeated
  # vowels, strip zero-width/dangling marks, reorder vowels) for noisy/OCR-derived text. Note
  # the returned tokens are then substrings of the *normalized* text, not the original.
  def self.segment(input_text, normalize: false)
    text = normalize ? std_normalize(input_text.to_s) : input_text
    decode_tokens(text, 'thapthim_segment')
  end

  # Syllable-level segmentation. Returns an array of syllable strings; their boundaries are a
  # superset of the word boundaries returned by +segment+. See +segment+ for +normalize:+.
  def self.syllables(input_text, normalize: false)
    text = normalize ? std_normalize(input_text.to_s) : input_text
    decode_tokens(text, 'thapthim_segment_syllables')
  end
end
