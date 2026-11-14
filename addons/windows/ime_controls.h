// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "tsf_compat.h"
#include <imm.h>
#include <array>
#include <cstdint>
#include <string>
inline const GUID conversionCompartmentId = {
    0xccf05dd8,
    0x4a87,
    0x11d7,
    {0xa6, 0xe2, 0x00, 0x06, 0x5b, 0x84, 0x43, 0x5c}};
inline const GUID languageBarId = {
    0x2c77a81e,
    0x41cc,
    0x4178,
    {0xa3, 0xa7, 0x5f, 0x8a, 0x98, 0x75, 0x68, 0xe6}};
inline const GUID systemTrayCategory = {
    0x25504fb4,
    0x7bab,
    0x4bc1,
    {0x9c, 0x69, 0xcf, 0x81, 0x89, 0x0f, 0x0e, 0xf5}};
inline const std::array<GUID, 3> registeredCategories = {
    GUID_TFCAT_TIP_KEYBOARD, GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    systemTrayCategory};
inline const GUID imeToggleId = {
    0xa8d5d721,
    0xbc04,
    0x40cb,
    {0xa1, 0x52, 0xba, 0x99, 0x7a, 0x4d, 0xc9, 0x20}};
inline const GUID imeOnId = {0xa8d5d722,
                             0xbc04,
                             0x40cb,
                             {0xa1, 0x52, 0xba, 0x99, 0x7a, 0x4d, 0xc9, 0x20}};
inline const GUID imeOffId = {0xa8d5d723,
                              0xbc04,
                              0x40cb,
                              {0xa1, 0x52, 0xba, 0x99, 0x7a, 0x4d, 0xc9, 0x20}};
struct ImePreservedKey {
  const GUID *id;
  TF_PRESERVEDKEY key;
};
inline constexpr ImePreservedKey imeKeys[] = {{&imeToggleId, {VK_KANJI, 0}},
                                              {&imeOnId, {VK_IME_ON, 0}},
                                              {&imeOffId, {VK_IME_OFF, 0}}};

struct ImeMode {
  const wchar_t *name;
  DWORD flags;
};
inline constexpr std::array<ImeMode, 5> imeModes = {
    {{L"ひらがな", IME_CMODE_NATIVE | IME_CMODE_FULLSHAPE},
     {L"カタカナ", IME_CMODE_NATIVE | IME_CMODE_KATAKANA | IME_CMODE_FULLSHAPE},
     {L"半角カタカナ", IME_CMODE_NATIVE | IME_CMODE_KATAKANA},
     {L"全角英数", IME_CMODE_FULLSHAPE},
     {L"ASCII", IME_CMODE_ALPHANUMERIC}}};
inline constexpr DWORD imeModeMask =
    IME_CMODE_NATIVE | IME_CMODE_KATAKANA | IME_CMODE_FULLSHAPE;
inline DWORD conversionFlags(uint8_t mode, DWORD previous = 0) {
  return (previous & ~imeModeMask) |
         imeModes[mode < imeModes.size() ? mode : 0].flags;
}
inline uint8_t modeFromFlags(DWORD flags, uint8_t previous) {
  const auto relevant = flags & imeModeMask;
  if (relevant == IME_CMODE_NATIVE)
    return 0;
  for (uint8_t mode = 0; mode < imeModes.size(); ++mode)
    if (imeModes[mode].flags == relevant)
      return mode;
  return previous;
}

inline HICON inputModeIcon(const std::wstring &label) {
  HICON icon = nullptr;
  const int width = GetSystemMetrics(SM_CXSMICON),
            height = GetSystemMetrics(SM_CYSMICON);
  if (width <= 0 || height <= 0)
    return nullptr;
  HDC screen = GetDC(nullptr);
  if (!screen)
    return nullptr;
  HDC colorDC = CreateCompatibleDC(screen), maskDC = CreateCompatibleDC(screen);
  HBITMAP color = CreateCompatibleBitmap(screen, width, height);
  HBITMAP mask = CreateBitmap(width, height, 1, 1, nullptr);
  ReleaseDC(nullptr, screen);
  if (colorDC && maskDC && color && mask) {
    auto oldColor = SelectObject(colorDC, color),
         oldMask = SelectObject(maskDC, mask);
    auto oldColorFont = SelectObject(colorDC, GetStockObject(DEFAULT_GUI_FONT));
    auto oldMaskFont = SelectObject(maskDC, GetStockObject(DEFAULT_GUI_FONT));
    PatBlt(colorDC, 0, 0, width, height, BLACKNESS);
    PatBlt(maskDC, 0, 0, width, height, WHITENESS);
    SetBkMode(colorDC, TRANSPARENT);
    SetBkMode(maskDC, TRANSPARENT);
    SetTextColor(colorDC, GetSysColor(COLOR_MENUTEXT));
    SetTextColor(maskDC, RGB(0, 0, 0));
    RECT area{0, 0, width, height};
    DrawTextW(colorDC, label.c_str(), -1, &area,
              DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    DrawTextW(maskDC, label.c_str(), -1, &area,
              DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    SelectObject(colorDC, oldColor);
    SelectObject(maskDC, oldMask);
    SelectObject(colorDC, oldColorFont);
    SelectObject(maskDC, oldMaskFont);
    ICONINFO info{TRUE, 0, 0, mask, color};
    icon = CreateIconIndirect(&info);
  }
  if (color)
    DeleteObject(color);
  if (mask)
    DeleteObject(mask);
  if (colorDC)
    DeleteDC(colorDC);
  if (maskDC)
    DeleteDC(maskDC);
  return icon;
}
