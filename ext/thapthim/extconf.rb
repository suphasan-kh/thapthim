require 'mkmf'
require 'rb_sys/mkmf'

# This helper from the rb-sys toolchain automatically sets up 
# a RubyGems-compatible Makefile for Rust extension library
create_rust_makefile('thapthim') do |config|
  # Forces the compiler to optimize the output binary heavily by default
  config.profile = :release 
end