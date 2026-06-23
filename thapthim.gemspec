# frozen_string_literal: true

require_relative "lib/thapthim/version"

Gem::Specification.new do |spec|
  spec.name = "thapthim"
  spec.version = Thapthim::VERSION
  spec.authors = ["suphasan-kh"]
  spec.email = ["suphasan2004@gmail.com"]

  spec.summary = "Thapthim: A Thai Text Processor in Ruby"
  spec.description = "A Thai text processor fully written in Ruby"
  spec.homepage = "https://github.com/suphasan-kh/thapthim"
  # Source code is MIT, but the bundled model assets derive from non-commercial corpora
  # (BEST: CC-BY-NC-SA-3.0; LST20: NECTEC non-commercial). Both apply — the gem as distributed
  # is non-commercial. See THIRD_PARTY_NOTICES.md. (Listed together so license scanners flag the
  # NonCommercial constraint; this is AND, not a choose-one.)
  spec.licenses = ["MIT", "CC-BY-NC-SA-3.0"]
  spec.required_ruby_version = ">= 3.2.0"
  # spec.metadata["allowed_push_host"] = "TODO: Set to your gem server 'https://example.com'"
  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = "https://github.com/suphasan-kh/thapthim"
  spec.metadata["changelog_uri"] = "https://github.com/suphasan-kh/thapthim"

  # Uncomment the line below to require MFA for gem pushes.
  # This helps protect your gem from supply chain attacks by ensuring
  # no one can publish a new version without multi-factor authentication.
  # See: https://guides.rubygems.org/mfa-requirement-opt-in/
  # spec.metadata["rubygems_mfa_required"] = "true"

  # Specify which files should be added to the gem when it is released.
  # The `git ls-files -z` loads the files in the RubyGem that have been added into git.
  gemspec = File.basename(__FILE__)
  spec.files = IO.popen(%w[git ls-files -z], chdir: __dir__, err: IO::NULL) do |ls|
    ls.readlines("\x0", chomp: true).reject do |f|
      (f == gemspec) ||
        f.start_with?(*%w[bin/ Gemfile .gitignore .rspec spec/ .standard.yml]) ||
        f.start_with?("target/") || f.start_with?("tmp/") ||
        f.start_with?("test/") || f.start_with?("datasets/") || f.start_with?("tools/") ||
        f.end_with?(".dylib") || f.end_with?(".so") || f.end_with?(".bundle")
    end
  end
  spec.bindir = "exe"
  spec.executables = spec.files.grep(%r{\Aexe/}) { |f| File.basename(f) }
  spec.require_paths = ["lib"]

  # Uncomment to register a new dependency of your gem
  # spec.add_dependency "example-gem", "~> 1.0"

  # For more information and examples about making a new gem, check out our
  # guide at: https://guides.rubygems.org/make-your-own-gem/
  
  # 1. Instructs RubyGems to execute your compilation file on installation
  spec.extensions = ["ext/thapthim/extconf.rb"]

  # 2. Ensures the compilation tool is present during the asset building phase
  spec.add_development_dependency "rb_sys", "~> 0.9"
  spec.add_development_dependency "rake-compiler", "~> 1.2"
end
