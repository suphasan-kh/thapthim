# frozen_string_literal: true

require "bundler/gem_tasks"
require "rake/testtask"
require "rb_sys/extensiontask"

# Fast correctness suite (excludes the long-running performance benchmark).
Rake::TestTask.new(:test) do |t|
  t.libs << "test" << "lib"
  t.test_files = FileList["test/test_*.rb"].exclude("test/test_performance.rb")
  t.warning = false
end

# Performance benchmark — slow (~45s); run on demand with `rake test:perf`.
Rake::TestTask.new("test:perf") do |t|
  t.libs << "test" << "lib"
  t.test_files = ["test/test_performance.rb"]
  t.warning = false
end

RbSys::ExtensionTask.new("thapthim", Bundler.load_gemspec("thapthim.gemspec")) do |ext|
  ext.lib_dir = "lib/thapthim"
  ext.source_pattern = "*.{rs,toml}"
end

task default: [:compile, :test]
