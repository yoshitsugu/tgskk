#include "account.h"
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
    require(canonicalPath(config.model) ==
                canonicalPath((directory / L"bin" / L"models" /
                               L"zenz-v3.2-small-Q5_K_M.gguf")
                                  .wstring()),
            "shared model fallback");
    std::filesystem::create_directories(directory);
    require(!config.skkservWithoutInputScope,
            "missing-scope server must default off");
    require(config.server == "Google", "portable build server default");
    const auto defaults = directory / L"bin" / L"windows-defaults.ini";
    std::ofstream(defaults) << "[General]\nGoogleLookup=0\n";
    const auto disabled = Config::load(directory.wstring(), config.broker);
    require(disabled.server == "Disabled" &&
                disabled.fingerprint != config.fingerprint,
            "installer opt-out changes the effective configuration");
    std::ofstream(directory / L"windows.ini") << "[General]\nSkkserv=Google\n";
    require(Config::load(directory.wstring(), config.broker).server == "Google",
            "saved user preference overrides the installer opt-out");
    std::ofstream(directory / L"SKK-JISYO.user", std::ios::binary)
        << "learned-dictionary";
    config.ai = false;
    config.server = "Custom";
    config.host = "localhost";
    config.port = 1179;
    config.encoding = "utf-8";
    config.inlineCandidates = 0;
    config.skkservWithoutInputScope = true;
    Config::save(config);
    auto loaded = Config::load(directory.wstring(), config.broker);
    require(!loaded.ai && loaded.server == "Custom" &&
                loaded.host == "localhost" && loaded.port == 1179 &&
                loaded.inlineCandidates == 0 && loaded.encoding == "utf-8" &&
                loaded.skkservWithoutInputScope,
            "settings round trip");
    const auto original = read(directory / L"windows.ini");
    std::ofstream(directory / L"windows.ini")
        << "[General]\nSkkservWithoutInputScope=false\n";
    require(!Config::load(directory.wstring(), config.broker)
                 .skkservWithoutInputScope,
            "explicit missing-scope server opt-out");
    std::ofstream(directory / L"windows.ini")
        << "[General]\nSkkservWithoutInputScope=invalid\n";
    bool invalidScopeSetting = false;
    try {
      (void)Config::load(directory.wstring(), config.broker);
    } catch (const std::exception &) {
      invalidScopeSetting = true;
    }
    require(invalidScopeSetting,
            "invalid missing-scope server setting accepted");
    std::ofstream(directory / L"windows.ini", std::ios::binary) << original;
    const auto userModels = directory / L"models";
    std::filesystem::create_directories(userModels);
    const auto unusedModel = userModels / L"zenz-v3.2-small-Q5_K_M.gguf";
    std::ofstream(unusedModel) << "inaccessible unused model fixture";
    HANDLE rawToken = nullptr;
    require(OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &rawToken),
            "query token");
    Handle token(rawToken);
    DWORD size = 0;
    GetTokenInformation(token.value, TokenUser, nullptr, 0, &size);
    std::vector<uint8_t> user(size);
    require(
        GetTokenInformation(token.value, TokenUser, user.data(), size, &size),
        "current user SID");
    const auto sid = reinterpret_cast<TOKEN_USER *>(user.data())->User.Sid;
    {
      test::FileAccess denyDirectory(userModels.wstring(), sid,
                                     FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES,
                                     NO_INHERITANCE, DENY_ACCESS);
      test::FileAccess denyFile(unusedModel.wstring(), sid,
                                FILE_READ_ATTRIBUTES, NO_INHERITANCE,
                                DENY_ACCESS);
      std::error_code error;
      (void)std::filesystem::is_regular_file(unusedModel, error);
      require(error == std::errc::permission_denied ||
                  error.value() == ERROR_ACCESS_DENIED,
              "fixture did not deny model attributes");
      std::ofstream(directory / L"windows.ini")
          << "[General]\nAI=false\nSkkserv=Disabled\n";
      require(!Config::load(directory.wstring(), config.broker).ai,
              "unused model blocked disabled AI");
      std::ofstream(directory / L"windows.ini")
          << "[General]\nAI=true\nModel=" << utf8(config.model) << "\n";
      require(Config::load(directory.wstring(), config.broker).model ==
                  config.model,
              "unused default model blocked an explicit model");
      require(denyFile.close() && denyDirectory.close(), "model ACL rollback");
    }
    std::ofstream(directory / L"windows.ini", std::ios::binary) << original;
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
