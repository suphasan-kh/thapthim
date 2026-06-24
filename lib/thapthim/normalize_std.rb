# SPDX-FileCopyrightText: 2016-2026 PyThaiNLP Project
# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0

require 'fiddle'

module Thapthim
  # Thai text normalization. The implementation now lives in Rust (ext/thapthim/src/normalize.rs)
  # so the Ruby and Python bindings share a single, byte-identical normalizer; this is a thin FFI
  # wrapper over `thapthim_normalize`. The native externs are declared in segment_tcc.rb (where the
  # library is dlloaded, which happens after this file in the load order — only the runtime call
  # below touches NativeBridge, by which point it exists).
  #
  # Behaviour (unchanged): strip zero-width chars, collapse duplicate spaces/marks, reorder
  # misordered vowel/tone sequences, drop dangling combining marks. Verified byte-identical to the
  # former pure-Ruby implementation across the LST20 corpus + crafted edge cases.
  def self.std_normalize(input)
    return input if input.nil?
    text = sanitize_input(input.to_s) # guarantee UTF-8, NUL-free (the C boundary is NUL-terminated)
    return "" if text.empty?

    text_pointer = Fiddle::Pointer.to_ptr(text)
    result_address = NativeBridge['thapthim_normalize'].call(text_pointer.to_i)
    return "" if result_address.nil? || result_address == 0

    # `to_s` reads the NUL-terminated C string into a fresh (binary) Ruby String, so the buffer is
    # safe to free immediately afterward.
    normalized = Fiddle::Pointer.new(result_address).to_s.force_encoding(Encoding::UTF_8)
    NativeBridge['thapthim_free_string'].call(result_address)
    normalized
  end
end
