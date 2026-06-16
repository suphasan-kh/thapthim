# lib/thapthim/segment_tcc.rb
require 'fiddle'
require 'fiddle/import'

module Thapthim
  module NativeBridge
    extend Fiddle::Importer
    
    ext = RbConfig::CONFIG['DLEXT']
    LIB_PATH = File.expand_path("libthapthim.#{ext}", __dir__)

    if File.exist?(LIB_PATH)
      dlload LIB_PATH
      extern 'int* thapthim_tcc_positions(const char*, int*)'
      extern 'void thapthim_free_array(int*, int)' # 🆕 Bind the memory cleanup hook
    end
  end

  def self.tcc_positions(input_text)
    return [0] if input_text.nil? || input_text.empty?

    size_buffer = Fiddle::Pointer.malloc(Fiddle::SIZEOF_INT)
    raw_array_ptr = NativeBridge.thapthim_tcc_positions(input_text, size_buffer)
    return [0] if raw_array_ptr.null?

    total_elements = size_buffer[0, Fiddle::SIZEOF_INT].unpack1('i')
    positions = raw_array_ptr[0, total_elements * Fiddle::SIZEOF_INT].unpack('i*')
    
    # ✅ FIXED: Frees the raw pointer allocation back to Rust immediately after unpacking
    NativeBridge.thapthim_free_array(raw_array_ptr, total_elements)
    
    positions
  end

  def self.tcc_segment(input_text)
    return [] if input_text.nil? || input_text.empty?
    
    binary_str = input_text.b 
    positions = tcc_positions(input_text)
    
    segments = []
    positions.each_cons(2) do |start_idx, end_idx|
      segments << binary_str[start_idx...end_idx].force_encoding('UTF-8')
    end
    
    segments
  end
end