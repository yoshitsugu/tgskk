#include "settings.h"
#include <fstream>
#include <iostream>
using namespace tgskk::windows;
static void require(bool valid, const char *message) {
  if (!valid)
    throw std::runtime_error(message);
}
int main() {
  HWND window = nullptr;
  std::filesystem::path directory;
  try {
    wchar_t temporary[MAX_PATH];
    require(GetTempPathW(MAX_PATH, temporary) != 0, "temporary directory");
    directory = std::filesystem::path(temporary) /
                (L"tgskk-settings-" + std::to_wstring(randomId()));
    auto config = Config::load(directory.wstring(),
                               (directory / L"tgskk-broker.exe").wstring());
    window = createSettingsWindow(GetModuleHandleW(nullptr), config, false);
    require(window != nullptr, "settings window");
    require(SendMessageW(GetDlgItem(window, IDC_SKKSERV_WITHOUT_SCOPE_CHECK),
                         BM_GETCHECK, 0, 0) == BST_UNCHECKED,
            "missing-scope server GUI default");
    SendMessageW(GetDlgItem(window, IDC_SKKSERV_WITHOUT_SCOPE_CHECK),
                 BM_SETCHECK, BST_CHECKED, 0);
    for (const auto id : {IDC_DICTIONARY_EDIT, IDC_AI_CHECK, IDC_PORT_EDIT,
                          IDC_SAVE, IDC_STATUS})
      require(GetDlgItem(window, id) != nullptr, "missing control");
    SetWindowTextW(GetDlgItem(window, IDC_INLINE_CANDIDATES_EDIT), L"7");
    SendMessageW(GetDlgItem(window, IDC_AI_CHECK), BM_SETCHECK, BST_UNCHECKED,
                 0);
    SendMessageW(window, WM_COMMAND, MAKEWPARAM(IDC_SAVE, BN_CLICKED), 0);
    auto saved = Config::load(directory.wstring(), config.broker);
    require(saved.inlineCandidates == 7 && !saved.ai && saved.skkservWithoutInputScope,
            "GUI did not save controls");
    require(GetWindowTextLengthW(GetDlgItem(window, IDC_STATUS)) > 0,
            "no save feedback");
    SetWindowTextW(GetDlgItem(window, IDC_PORT_EDIT), L"70000");
    SendMessageW(window, WM_COMMAND, MAKEWPARAM(IDC_SAVE, BN_CLICKED), 0);
    require(Config::load(directory.wstring(), config.broker).port == 1178,
            "GUI silently truncated port");
    SetWindowTextW(GetDlgItem(window, IDC_PORT_EDIT), L"12x");
    SendMessageW(window, WM_COMMAND, MAKEWPARAM(IDC_SAVE, BN_CLICKED), 0);
    require(Config::load(directory.wstring(), config.broker).port == 1178,
            "GUI accepted invalid port suffix");
    DestroyWindow(window);
    window = nullptr;
    std::filesystem::remove_all(directory);
    return 0;
  } catch (const std::exception &error) {
    if (window)
      DestroyWindow(window);
    std::cerr << error.what() << '\n';
    if (!directory.empty()) {
      std::error_code ignored;
      std::filesystem::remove_all(directory, ignored);
    }
    return 1;
  }
}
