#include "model_verifier.hpp"

#include "verified_model_assets.hpp"

#include <CommonCrypto/CommonDigest.h>

#include <array>
#include <fstream>
#include <iomanip>
#include <sstream>
#include <system_error>

namespace outflow::mic_probe {
namespace {

std::string sha256_file(const std::filesystem::path& path, bool& ok) {
  CC_SHA256_CTX context;
  if (CC_SHA256_Init(&context) != 1) {
    ok = false;
    return {};
  }
  std::ifstream input(path, std::ios::binary);
  if (!input) {
    ok = false;
    return {};
  }
  std::array<char, 64 * 1024> chunk{};
  while (input) {
    input.read(chunk.data(), static_cast<std::streamsize>(chunk.size()));
    const auto count = input.gcount();
    if (count > 0 &&
        CC_SHA256_Update(&context, chunk.data(), static_cast<CC_LONG>(count)) != 1) {
      ok = false;
      return {};
    }
  }
  if (!input.eof()) {
    ok = false;
    return {};
  }
  std::array<unsigned char, CC_SHA256_DIGEST_LENGTH> digest{};
  if (CC_SHA256_Final(digest.data(), &context) != 1) {
    ok = false;
    return {};
  }
  std::ostringstream text;
  text << std::hex << std::setfill('0');
  for (const auto byte : digest) text << std::setw(2) << static_cast<unsigned>(byte);
  ok = true;
  return text.str();
}

}  // namespace

bool verify_model_directory(const std::filesystem::path& model_directory,
                            std::string& error) noexcept {
  try {
    const auto root_status = std::filesystem::symlink_status(model_directory);
    if (!std::filesystem::is_directory(root_status)) {
      error = "model directory is missing or is a symbolic link";
      return false;
    }
    for (const auto& asset : kVerifiedModelAssets) {
      const auto path = model_directory / asset.filename;
      const auto status = std::filesystem::symlink_status(path);
      if (!std::filesystem::is_regular_file(status)) {
        error = "model file is missing or is not a regular file: " +
                std::string(asset.filename);
        return false;
      }
      bool read_ok = false;
      const auto digest = sha256_file(path, read_ok);
      if (!read_ok || digest != asset.sha256) {
        error = "model file checksum mismatch: " + std::string(asset.filename);
        return false;
      }
    }
    return true;
  } catch (const std::filesystem::filesystem_error&) {
    error = "model asset metadata could not be read";
  } catch (const std::system_error&) {
    error = "model asset could not be verified";
  } catch (...) {
    error = "model asset verification failed";
  }
  return false;
}

}  // namespace outflow::mic_probe
