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

  def self.tcc_positions(input_text)
    return [0] if input_text.nil? || input_text.empty?

    text_pointer = Fiddle::Pointer.to_ptr(input_text.to_s)
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
    return [] if input_text.nil? || input_text.empty?
    
    positions = tcc_positions(input_text)
    
    segments = []
    positions.each_cons(2) do |start_idx, end_idx|
      segments << input_text[start_idx...end_idx]
    end
    
    segments
  end
end