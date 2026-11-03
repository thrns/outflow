#pragma once

#include <filesystem>
#include <string>

namespace outflow::mic_probe {

bool verify_model_directory(const std::filesystem::path& model_directory,
                            std::string& error) noexcept;

}  // namespace outflow::mic_probe
