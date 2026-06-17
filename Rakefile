# frozen_string_literal: true

require "bundler/gem_tasks"
require "rspec/core/rake_task"
require "rb_sys/extensiontask"

RSpec::Core::RakeTask.new(:spec)

RbSys::ExtensionTask.new("thapthim", Bundler.load_gemspec("thapthim.gemspec")) do |ext|
  ext.lib_dir = "lib/thapthim"
  ext.source_pattern = "*.{rs,toml}"
end

task default: [:compile, :spec]