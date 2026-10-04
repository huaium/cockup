require "digest"
require "fileutils"
require "open3"
require "rbconfig"
require "tmpdir"

updater = File.expand_path("update_homebrew.rb", __dir__)
targets = %w[aarch64-apple-darwin x86_64-apple-darwin]
Dir.mktmpdir do |dir|
  formula = File.join(dir, "cockup.rb")
  original = "class Cockup < Formula\n  version = \"0.2.1\"\n" + targets.map do |target|
    "  url \"https://github.com/huaium/cockup/releases/download/v\#{version}/cockup-v\#{version}-#{target}.tar.gz\"\n  sha256 \"#{'a' * 64}\"\n"
  end.join + "  license \"MIT\"\n  # Retain customizations\nend\n"
  File.write(formula, original)
  targets.each do |target|
    archive = "cockup-v0.2.2-#{target}.tar.gz"
    File.write(File.join(dir, archive), target)
    File.write(File.join(dir, "#{archive}.sha256"), "#{Digest::SHA256.hexdigest(target)}  #{archive}\n")
  end
  run = ->(tag) { Open3.capture3(RbConfig.ruby, updater, tag, formula, dir) }
  # A bad second asset must not partially update the first architecture.
  checksum = File.join(dir, "cockup-v0.2.2-x86_64-apple-darwin.tar.gz.sha256")
  correct = File.read(checksum)
  File.write(checksum, "invalid")
  _, _, status = run.call("v0.2.2")
  raise "Accepted invalid checksum or changed formula" if status.success? || File.read(formula) != original
  File.write(checksum, correct)
  _, stderr, status = run.call("v0.2.2")
  raise stderr unless status.success?
  updated = File.read(formula)
  raise "Missing version variable" unless updated.scan(/^  version = "0\.2\.2"$/).length == 1
  raise "Changed URL templates" unless updated.lines.grep(/url /) == original.lines.grep(/url /)
  raise "Lost customizations" unless updated.include?("Retain customizations")
  raise "Added redundant version directive" if updated.match?(/^\s*version "/)
  targets.each { |target| raise "Wrong checksum" unless updated.include?(Digest::SHA256.hexdigest(target)) }
  _, stderr, status = run.call("v0.2.2")
  raise "Not idempotent: #{stderr}" unless status.success? && File.read(formula) == updated
  ["v0.2.1", "invalid"].each do |tag|
    _, _, status = run.call(tag)
    raise "Accepted downgrade/invalid tag" if status.success? || File.read(formula) != updated
  end
  [original.sub(/^  version = .*\n/, ""),
   original.sub('  version = "0.2.1"', "  version = \"0.2.1\"\n  version = \"0.2.1\""),
   original.sub('cockup-v#{version}-x86_64', 'other-v#{version}-x86_64')].each do |invalid|
    File.write(formula, invalid)
    _, _, status = run.call("v0.2.2")
    raise "Accepted malformed formula or changed it" if status.success? || File.read(formula) != invalid
  end
end
puts "Homebrew updater checks passed"
