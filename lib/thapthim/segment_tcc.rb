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
      # Declare as generic void pointers to make Fiddle's internal tables happy
      extern 'void* thapthim_tcc_positions(void*, void*)'
      extern 'void thapthim_free_array(void*, int)'
    else
      raise LoadError, "Thapthim Core Failure: Native binary missing at #{LIB_PATH}"
    end
  end

  def self.tcc_positions(input_text)
    return [0] if input_text.nil? || input_text.empty?

    # 1. Convert the Ruby string into a distinct, safe C-string Pointer
    text_pointer = Fiddle::Pointer.to_ptr(input_text.to_s)

    # 2. Allocate an explicit memory segment for the returned integer size
    size_buffer = Fiddle::Pointer.malloc(Fiddle::SIZEOF_INT)
    
    # 3. Pull the direct function reference handles
    func = NativeBridge['thapthim_tcc_positions']
    
    # 4. Pass the RAW memory addresses (integers) to guarantee Fiddle doesn't try to auto-cast
    raw_array_address = func.call(text_pointer.to_i, size_buffer.to_i)
    return [0] if raw_array_address == 0 || raw_array_address.nil?

    # 5. Turn the returned raw memory address back into a readable pointer wrapper
    raw_array_ptr = Fiddle::Pointer.new(raw_array_address)

    # 6. Unpack the dimensions and values safely
    total_elements = size_buffer[0, Fiddle::SIZEOF_INT].unpack1('i')
    positions = raw_array_ptr[0, total_elements * Fiddle::SIZEOF_INT].unpack('i*')
    
    # 7. Release the raw Rust allocations cleanly to prevent RAM leaks
    free_func = NativeBridge['thapthim_free_array']
    free_func.call(raw_array_ptr.to_i, total_elements)
    
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