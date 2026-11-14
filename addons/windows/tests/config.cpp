#include "broker_platform.h"
#include <fstream>
#include <iostream>
#include <iterator>
using namespace tgskk::windows;
static void require(bool value, const char *message) {
  if (!value)
    throw std::runtime_error(message);
}
static std::string read(const std::filesystem::path &path) {
  std::ifstream input(path, std::ios::binary);
  return {std::istreambuf_iterator<char>(input), {}};
}
int main() {
  std::filesystem::path directory;
  try {
    wchar_t temporary[MAX_PATH];
    require(GetTempPathW(MAX_PATH, temporary) != 0, "temporary path");
    directory = std::filesystem::path(temporary) /
                (L"tgskk-config-" + std::to_wstring(randomId()));
    std::filesystem::create_directories(directory / L"bin" / L"models");
    std::ofstream(directory / L"bin" / L"models" /
                  L"zenz-v3.2-small-Q5_K_M.gguf")
        << "model fixture";
    auto config =
        Config::load(directory.wstring(),
                     (directory / L"bin" / L"tgskk-broker.exe").wstring());
    require(config.model == (directory / L"bin" / L"models" /
                             L"zenz-v3.2-small-Q5_K_M.gguf")
                                .wstring(),
            "shared model fallback");
    std::filesystem::create_directories(directory);
    std::ofstream(directory / L"SKK-JISYO.user", std::ios::binary)
        << "learned-dictionary";
    config.ai = false;
    config.server = "Custom";
    config.host = "localhost";
    config.port = 1179;
    config.encoding = "utf-8";
    config.inlineCandidates = 0;
    Config::save(config);
    auto loaded = Config::load(directory.wstring(), config.broker);
    require(!loaded.ai && loaded.server == "Custom" &&
                loaded.host == "localhost" && loaded.port == 1179 &&
                loaded.inlineCandidates == 0 && loaded.encoding == "utf-8",
            "settings round trip");
    std::ofstream(directory / L"models") << "block unused default model lookup";
    require(Config::load(directory.wstring(), config.broker).model ==
                config.model,
            "explicit model was blocked by an unused default path");
    const auto original = read(directory / L"windows.ini");
    for (int fault = 0; fault < 4; ++fault) {
      auto invalid = config;
      if (fault == 0)
        invalid.inlineCandidates = 65;
      if (fault == 1)
        invalid.port = 0;
      if (fault == 2)
        invalid.dictionary += L"\nAI=true";
      if (fault == 3)
        invalid.server = "unknown";
      bool rejected = false;
      try {
        Config::save(invalid);
      } catch (const std::exception &) {
        rejected = true;
      }
      require(rejected && read(directory / L"windows.ini") == original,
              "invalid settings replaced the old file");
    }
    {
      Handle lock(CreateFileW((directory / L"windows.ini").c_str(),
                              GENERIC_READ, 0, nullptr, OPEN_EXISTING,
                              FILE_ATTRIBUTE_NORMAL, nullptr));
      require(bool(lock), "lock settings");
      bool rejected = false;
      try {
        Config::save(config);
      } catch (const std::exception &) {
        rejected = true;
      }
      require(rejected, "replacement ignored a locked settings file");
    }
    require(read(directory / L"windows.ini") == original,
            "failed replacement changed settings");
    for (const auto &entry : std::filesystem::directory_iterator(directory))
      require(entry.path().extension() != L".tmp", "temporary settings leaked");
    config.inlineCandidates = 7;
    Config::save(config);
    require(Config::load(directory.wstring(), config.broker).inlineCandidates ==
                7,
            "save did not recover");
    require(read(directory / L"SKK-JISYO.user") == "learned-dictionary",
            "saving settings changed learning");
    std::filesystem::remove_all(directory);
    return 0;
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    if (!directory.empty()) {
      std::error_code ignored;
      std::filesystem::remove_all(directory, ignored);
    }
    return 1;
  }
}
