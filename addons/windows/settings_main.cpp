// SPDX-License-Identifier: GPL-3.0-or-later
#include "broker_platform.h"
#include "settings.h"
#include <combaseapi.h>
#include <stdexcept>

int APIENTRY wWinMain(HINSTANCE instance, HINSTANCE, LPWSTR, int) {
  bool initialized = false;
  try {
    HRESULT hr = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    if (FAILED(hr))
      throw std::runtime_error("CoInitializeEx failed");

    initialized = true;
    const auto self = tgskk::windows::executablePath();
    const auto broker =
        (std::filesystem::path(self).parent_path() / L"tgskk-broker.exe")
            .wstring();
    const auto config =
        tgskk::windows::Config::load(tgskk::windows::homeDirectory(), broker);

    HWND hwnd = tgskk::windows::createSettingsWindow(instance, config);
    if (!hwnd)
      throw std::runtime_error("createSettingsWindow failed");

    MSG msg{};
    int result = 0;
    while ((result = GetMessageW(&msg, nullptr, 0, 0)) > 0) {
      if (!IsDialogMessageW(hwnd, &msg)) {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
      }
    }

    if (result < 0)
      throw std::runtime_error("GetMessage failed");
    CoUninitialize();
    initialized = false;
    return static_cast<int>(msg.wParam);
  } catch (const std::exception &e) {
    if (initialized)
      CoUninitialize();
    MessageBoxW(nullptr,
                L"設定を開けませんでした。windows.ini "
                L"の内容とアクセス権を確認してください。",
                L"tgskk 設定エラー", MB_OK | MB_ICONERROR);
    return 1;
  }
}
