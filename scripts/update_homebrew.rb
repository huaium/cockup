require "digest"
require "tempfile"

# Update only Cockup's architecture-specific URLs/checksums; retain tap customizations.
tag, formula_path, asset_dir = ARGV
abort "Usage: ruby scripts/update_homebrew.rb vX.Y.Z FORMULA ASSET_DIR" unless
  ARGV.length == 3 && tag.match?(/\Av(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)\z/)

formula = File.read(formula_path)
versions = formula.scan(%r{releases/download/v(\d+\.\d+\.\d+)/}).flatten.uniq
abort "Expected one Cockup version in formula" unless versions.length == 1
version = tag.delete_prefix("v")
if (versions.first.split(".").map(&:to_i) <=> version.split(".").map(&:to_i)) == 1
  abort "Refusing to downgrade formula from #{versions.first} to #{version}"
end

%w[aarch64-apple-darwin x86_64-apple-darwin].each do |target|
  archive = "cockup-#{tag}-#{target}.tar.gz"
  path = File.join(asset_dir, archive)
  checksum = Digest::SHA256.file(path).hexdigest
  expected = File.read("#{path}.sha256").strip
  abort "Checksum mismatch for #{archive}" unless expected.match?(/\A#{checksum}\s+\*?#{Regexp.escape(archive)}\z/)
  url = "https://github.com/huaium/cockup/releases/download/#{tag}/#{archive}"
  pattern = /url "https:\/\/github\.com\/huaium\/cockup\/releases\/download\/v[\d.]+\/cockup-v[\d.]+-#{Regexp.escape(target)}\.tar\.gz"\n(\s*)sha256 "[a-f0-9]{64}"/
  abort "Expected exactly one URL/checksum pair for #{target}" unless formula.scan(pattern).length == 1
  formula = formula.sub(pattern) { "url \"#{url}\"\n#{Regexp.last_match(1)}sha256 \"#{checksum}\"" }
end

if formula == File.read(formula_path)
  puts "Formula already matches #{tag}"
  exit
end
Tempfile.create(["cockup-formula", ".rb"], File.dirname(formula_path)) do |file|
  file.write(formula)
  file.flush
  file.fsync
  File.chmod(File.stat(formula_path).mode & 0o777, file.path)
  File.rename(file.path, formula_path)
end
puts "Updated #{formula_path} to #{tag}"
