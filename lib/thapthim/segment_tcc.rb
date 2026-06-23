# lib/thapthim/segment_tcc.rb
require 'fiddle'
require 'fiddle/import'

module Thapthim
  module NativeBridge
    extend Fiddle::Importer
    
    ext = RbConfig::CONFIG['DLEXT']
    LIB_PATH = File.expand_path("thapthim.#{ext}", __dir__)

    if File.exist?(LIB_PATH)
      dlload LIB_PATH
      extern 'void* thapthim_tcc_positions(void*, void*)'
      extern 'void thapthim_free_array(void*, int)'
    else
      raise LoadError, "Thapthim Core Failure: Native binary missing at #{LIB_PATH}"
    end
  end

  # Normalize arbitrary user input into a clean UTF-8 string the native layer can consume
  # without silent truncation or total data loss. Handles: non-String input (coerced via
  # +to_s+), non-UTF-8 encodings (transcoded — e.g. TIS-620/Windows-874), UTF-8 carrying
  # invalid bytes (scrubbed to U+FFFD), and embedded NULs (stripped — the native boundary is
  # NUL-terminated, so a NUL would otherwise truncate everything after it). Always returns a
  # valid-UTF-8, NUL-free String. Callers MUST slice/index the returned string (not the
  # original input) so the byte/char offsets from the native layer stay consistent.
  def self.sanitize_input(input)
    str = input.is_a?(String) ? input : input.to_s

    unless str.encoding == Encoding::UTF_8
      str =
        if str.encoding == Encoding::ASCII_8BIT
          # Binary: prefer a UTF-8 reinterpretation (bytes are often already UTF-8),
          # otherwise scrub the undecodable bytes.
          reinterpreted = str.dup.force_encoding(Encoding::UTF_8)
          reinterpreted.valid_encoding? ? reinterpreted : reinterpreted.scrub
        else
          # Known encoding (TIS-620, Windows-874, …): transcode into UTF-8.
          str.encode(Encoding::UTF_8, invalid: :replace, undef: :replace)
        end
    end

    str = str.scrub unless str.valid_encoding?
    nul = 0.chr
    str = str.delete(nul) if str.include?(nul)
    str
  end
  private_class_method :sanitize_input

  def self.tcc_positions(input_text)
    text = sanitize_input(input_text)
    return [0] if text.empty?

    text_pointer = Fiddle::Pointer.to_ptr(text)
    size_buffer = Fiddle::Pointer.malloc(Fiddle::SIZEOF_INT)
    
    func = NativeBridge['thapthim_tcc_positions']
    raw_array_address = func.call(text_pointer.to_i, size_buffer.to_i)
    return [0] if raw_array_address == 0 || raw_array_address.nil?

    raw_array_ptr = Fiddle::Pointer.new(raw_array_address)

    total_elements = size_buffer[0, Fiddle::SIZEOF_INT].unpack1('i')
    positions = raw_array_ptr[0, total_elements * Fiddle::SIZEOF_INT].unpack('i*')
    
    free_func = NativeBridge['thapthim_free_array']
    free_func.call(raw_array_ptr.to_i, total_elements)
    
    positions
  end

  def self.tcc_segment(input_text)
    text = sanitize_input(input_text)
    return [] if text.empty?

    positions = tcc_positions(text)

    segments = []
    positions.each_cons(2) do |start_idx, end_idx|
      segments << text[start_idx...end_idx]
    end

    segments
  end
end