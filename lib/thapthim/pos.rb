# lib/thapthim/pos.rb
require 'fiddle'

module Thapthim
  # Reopen the bridge created in segment_tcc.rb (the library is already dlloaded there) and attach
  # the POS-tagging entry point. Tokens cross the C boundary as one concatenated UTF-8 buffer plus a
  # per-token byte-length array (a delimiter is unusable — a whitespace token may itself be a space
  # or newline); the native side returns one tag id per token.
  module NativeBridge
    extern 'void* thapthim_pos_tag(void*, void*, int, void*)'
    extern 'void thapthim_free_u8_array(void*, int)'
  end

  # LST20 POS tags in the dense id order defined by `Pos::ALL` on the Rust side. The native tagger
  # returns these ids; index into this table to recover the tag string. Keep in lockstep with the
  # Rust enum order or the model must be retrained.
  POS_TAGS = %w[NN VV PU CC PS AX AV FX NU AJ CL PR NG PA XX IJ].freeze

  # Part-of-speech tag a Thai sentence. Cascade, not joint — two input modes:
  #   * String  → segmented with +word_segment+ first (keep_whitespace: true, so the space token
  #               that LST20 tags PU is preserved as in training), then tagged.
  #   * Array   → treated as already-segmented tokens and tagged directly. Use this for evaluation on
  #               gold tokens so a segmentation error never contaminates the POS result.
  #
  # Returns an array of <tt>[surface, tag]</tt> pairs (+tag+ a String from +POS_TAGS+). +normalize:+
  # applies only to the String path (it is forwarded to +word_segment+). Empty input → +[]+.
  #
  # Requires a trained model: the native library loads it once from the +THAPTHIM_POS_MODEL+ env var
  # (a +pos_hmm.bin+ built by <tt>cargo run --release --example train_pos_hmm</tt>).
  def self.pos_tag(input, normalize: false)
    tokens =
      if input.is_a?(Array)
        input.map { |t| sanitize_input(t) }
      else
        word_segment(input, normalize: normalize, keep_whitespace: true)
      end
    return [] if tokens.empty?

    # Compute lengths from the exact strings we concatenate, so byte offsets stay in sync on the Rust
    # side. word_segment tokens are already sanitized substrings; array tokens were sanitized above.
    concat = tokens.join
    lengths = tokens.map(&:bytesize).pack('l*') # 32-bit signed, matches the C `int` lengths array

    concat_ptr = Fiddle::Pointer.to_ptr(concat)
    lengths_ptr = Fiddle::Pointer.to_ptr(lengths)
    # RUBY_FREE so the out-param buffer is reclaimed at GC (see segment.rb for the rationale).
    size_buffer = Fiddle::Pointer.malloc(Fiddle::SIZEOF_INT, Fiddle::RUBY_FREE)

    raw_address = NativeBridge['thapthim_pos_tag']
                  .call(concat_ptr.to_i, lengths_ptr.to_i, tokens.length, size_buffer.to_i)
    return tokens.map { |t| [t, nil] } if raw_address.nil? || raw_address == 0

    raw_ptr = Fiddle::Pointer.new(raw_address)
    count = size_buffer[0, Fiddle::SIZEOF_INT].unpack1('i')
    begin
      ids = count.zero? ? [] : raw_ptr[0, count].unpack('C*')
    ensure
      # Count is read before this block; the ensure keeps the Rust buffer from leaking on a raise.
      NativeBridge['thapthim_free_u8_array'].call(raw_ptr.to_i, count)
    end

    tokens.zip(ids.map { |i| POS_TAGS[i] })
  end
end
