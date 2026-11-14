// SPDX-License-Identifier: GPL-3.0-or-later
#include "settings.h"
#include <algorithm>
#include <commdlg.h>
#include <string>

namespace tgskk::windows {
namespace {

constexpr int kMargin = 12;
constexpr int kLabelW = 170;
constexpr int kEditW = 250;
constexpr int kBtnW = 60;
constexpr int kH = 22;
constexpr int kGap = 8;
constexpr int kRow = kH + kGap;
constexpr int kClientW = 550;

inline int row(int n) { return kMargin + n * kRow; }

struct State {
  HINSTANCE instance;
  Config config;
  HFONT font = nullptr;
  int scroll = 0;
};

static HWND addLabel(HWND parent, HINSTANCE inst, int id, const wchar_t *text,
                     int x, int y, int w, int h) {
  return CreateWindowExW(
      0, L"STATIC", text, WS_CHILD | WS_VISIBLE | SS_LEFT, x, y, w, h, parent,
      reinterpret_cast<HMENU>(static_cast<INT_PTR>(id)), inst, nullptr);
}

static HWND addEdit(HWND parent, HINSTANCE inst, int id, const wchar_t *text,
                    int x, int y, int w, int h, DWORD style = 0) {
  return CreateWindowExW(
      WS_EX_CLIENTEDGE, L"EDIT", text,
      WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL | style, x, y, w, h,
      parent, reinterpret_cast<HMENU>(static_cast<INT_PTR>(id)), inst, nullptr);
}

static HWND addButton(HWND parent, HINSTANCE inst, int id, const wchar_t *text,
                      int x, int y, int w, int h, DWORD style = BS_PUSHBUTTON) {
  return CreateWindowExW(
      0, L"BUTTON", text, WS_CHILD | WS_VISIBLE | WS_TABSTOP | style, x, y, w,
      h, parent, reinterpret_cast<HMENU>(static_cast<INT_PTR>(id)), inst,
      nullptr);
}

static HWND addCombo(HWND parent, HINSTANCE inst, int id, int x, int y, int w,
                     int h) {
  return CreateWindowExW(
      0, L"COMBOBOX", nullptr,
      WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST | WS_VSCROLL | WS_TABSTOP, x, y,
      w, h, parent, reinterpret_cast<HMENU>(static_cast<INT_PTR>(id)), inst,
      nullptr);
}

static void setFont(HWND hwnd, HFONT font) {
  if (font)
    SendMessageW(hwnd, WM_SETFONT, reinterpret_cast<WPARAM>(font), 0);
}

static std::wstring getWindowText(HWND hwnd) {
  int len = GetWindowTextLengthW(hwnd);
  std::wstring s(len, L'\0');
  if (len > 0)
    GetWindowTextW(hwnd, s.data(), len + 1);
  s.resize(len);
  return s;
}

static void browseOpen(HWND owner, int editId, const wchar_t *filter) {
  wchar_t path[32768]{};
  HWND edit = GetDlgItem(owner, editId);
  GetWindowTextW(edit, path, 32768);
  OPENFILENAMEW ofn{};
  ofn.lStructSize = sizeof(ofn);
  ofn.hwndOwner = owner;
  ofn.lpstrFile = path;
  ofn.nMaxFile = 32768;
  ofn.lpstrFilter = filter;
  ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY;
  if (GetOpenFileNameW(&ofn))
    SetWindowTextW(edit, path);
}

static void browseSave(HWND owner, int editId, const wchar_t *filter) {
  wchar_t path[32768]{};
  HWND edit = GetDlgItem(owner, editId);
  GetWindowTextW(edit, path, 32768);
  OPENFILENAMEW ofn{};
  ofn.lStructSize = sizeof(ofn);
  ofn.hwndOwner = owner;
  ofn.lpstrFile = path;
  ofn.nMaxFile = 32768;
  ofn.lpstrFilter = filter;
  ofn.Flags = OFN_PATHMUSTEXIST | OFN_HIDEREADONLY;
  if (GetSaveFileNameW(&ofn))
    SetWindowTextW(edit, path);
}

static unsigned long number(const std::wstring &text, unsigned long minimum,
                            unsigned long maximum) {
  if (text.empty() ||
      text.find_first_not_of(L"0123456789") != std::wstring::npos)
    throw std::runtime_error("Enter a whole number");
  const auto value = std::stoul(text);
  if (value < minimum || value > maximum)
    throw std::runtime_error("Number is outside the allowed range");
  return value;
}
static void doSave(HWND hwnd, State *state) {
  try {
    Config c = state->config;

    c.dictionary = getWindowText(GetDlgItem(hwnd, IDC_DICTIONARY_EDIT));

    int encIdx = static_cast<int>(
        SendMessageW(GetDlgItem(hwnd, IDC_ENCODING_COMBO), CB_GETCURSEL, 0, 0));
    c.encoding = (encIdx == 1) ? "utf-8" : "euc-jp";

    c.userDictionary =
        getWindowText(GetDlgItem(hwnd, IDC_USER_DICTIONARY_EDIT));

    c.ai = SendMessageW(GetDlgItem(hwnd, IDC_AI_CHECK), BM_GETCHECK, 0, 0) ==
           BST_CHECKED;

    c.worker = getWindowText(GetDlgItem(hwnd, IDC_WORKER_EDIT));
    c.model = getWindowText(GetDlgItem(hwnd, IDC_MODEL_EDIT));

    int srvIdx = static_cast<int>(
        SendMessageW(GetDlgItem(hwnd, IDC_SKKSERV_COMBO), CB_GETCURSEL, 0, 0));
    if (srvIdx == 0)
      c.server = "Google";
    else if (srvIdx == 1)
      c.server = "Custom";
    else
      c.server = "Disabled";

    c.helper = getWindowText(GetDlgItem(hwnd, IDC_HELPER_EDIT));
    c.skkservWithoutInputScope = SendMessageW(
        GetDlgItem(hwnd, IDC_SKKSERV_WITHOUT_SCOPE_CHECK), BM_GETCHECK, 0, 0) == BST_CHECKED;
    c.host = utf8(getWindowText(GetDlgItem(hwnd, IDC_HOST_EDIT)));

    auto portText = getWindowText(GetDlgItem(hwnd, IDC_PORT_EDIT));
    c.port = static_cast<uint16_t>(number(portText, 1, 65535));

    int srvEncIdx = static_cast<int>(SendMessageW(
        GetDlgItem(hwnd, IDC_SERVER_ENCODING_COMBO), CB_GETCURSEL, 0, 0));
    c.serverEncoding = (srvEncIdx == 1) ? "utf-8" : "euc-jp";

    auto inlineText =
        getWindowText(GetDlgItem(hwnd, IDC_INLINE_CANDIDATES_EDIT));
    c.inlineCandidates = static_cast<uint32_t>(number(inlineText, 0, 64));

    Config::save(c);

    SetWindowTextW(GetDlgItem(hwnd, IDC_STATUS),
                   L"設定を保存しました。Windowsからサインアウトして、再度サイ"
                   L"ンインすると反映されます。");
  } catch (const std::exception &e) {
    try {
      SetWindowTextW(GetDlgItem(hwnd, IDC_STATUS), wide(e.what()).c_str());
    } catch (...) {
      SetWindowTextW(GetDlgItem(hwnd, IDC_STATUS), L"保存できませんでした。");
    }
  } catch (...) {
    SetWindowTextW(GetDlgItem(hwnd, IDC_STATUS), L"保存に失敗しました。");
  }
}

static void createControls(HWND hwnd, State *state) {
  const int xLabel = kMargin;
  const int xEdit = kMargin + kLabelW + kGap;
  const int xBtn = xEdit + kEditW + kGap;

  int y = row(0);
  addLabel(hwnd, state->instance, IDC_DICTIONARY_LABEL, L"辞書", xLabel, y,
           kLabelW, kH);
  addEdit(hwnd, state->instance, IDC_DICTIONARY_EDIT,
          state->config.dictionary.c_str(), xEdit, y, kEditW, kH);
  addButton(hwnd, state->instance, IDC_DICTIONARY_BROWSE, L"参照…", xBtn, y,
            kBtnW, kH);

  y = row(1);
  addLabel(hwnd, state->instance, IDC_ENCODING_LABEL, L"辞書の文字コード",
           xLabel, y, kLabelW, kH);
  HWND enc = addCombo(hwnd, state->instance, IDC_ENCODING_COMBO, xEdit, y, 160,
                      kH * 5);
  SendMessageW(enc, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(L"euc-jp"));
  SendMessageW(enc, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(L"utf-8"));

  y = row(2);
  addLabel(hwnd, state->instance, IDC_USER_DICTIONARY_LABEL, L"ユーザー辞書",
           xLabel, y, kLabelW, kH);
  addEdit(hwnd, state->instance, IDC_USER_DICTIONARY_EDIT,
          state->config.userDictionary.c_str(), xEdit, y, kEditW, kH);
  addButton(hwnd, state->instance, IDC_USER_DICTIONARY_BROWSE, L"参照…", xBtn,
            y, kBtnW, kH);

  y = row(3);
  addButton(hwnd, state->instance, IDC_AI_CHECK, L"AIによる候補の並べ替え",
            xEdit, y, 200, kH, BS_AUTOCHECKBOX);

  y = row(4);
  addLabel(hwnd, state->instance, IDC_WORKER_LABEL, L"AI実行ファイル", xLabel,
           y, kLabelW, kH);
  addEdit(hwnd, state->instance, IDC_WORKER_EDIT, state->config.worker.c_str(),
          xEdit, y, kEditW, kH);
  addButton(hwnd, state->instance, IDC_WORKER_BROWSE, L"参照…", xBtn, y, kBtnW,
            kH);

  y = row(5);
  addLabel(hwnd, state->instance, IDC_MODEL_LABEL, L"モデルファイル", xLabel, y,
           kLabelW, kH);
  addEdit(hwnd, state->instance, IDC_MODEL_EDIT, state->config.model.c_str(),
          xEdit, y, kEditW, kH);
  addButton(hwnd, state->instance, IDC_MODEL_BROWSE, L"参照…", xBtn, y, kBtnW,
            kH);

  y = row(6);
  addLabel(hwnd, state->instance, IDC_SKKSERV_LABEL, L"SKKサーバ", xLabel, y,
           kLabelW, kH);
  HWND srv =
      addCombo(hwnd, state->instance, IDC_SKKSERV_COMBO, xEdit, y, 160, kH * 5);
  SendMessageW(srv, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(L"Google"));
  SendMessageW(srv, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(L"Custom"));
  SendMessageW(srv, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(L"Disabled"));

  y = row(7);
  addLabel(hwnd, state->instance, IDC_HELPER_LABEL, L"Google連携実行ファイル",
           xLabel, y, kLabelW, kH);
  addEdit(hwnd, state->instance, IDC_HELPER_EDIT, state->config.helper.c_str(),
          xEdit, y, kEditW, kH);
  addButton(hwnd, state->instance, IDC_HELPER_BROWSE, L"参照…", xBtn, y, kBtnW,
            kH);

  y = row(8);
  addLabel(hwnd, state->instance, IDC_HOST_LABEL, L"ホスト", xLabel, y, kLabelW,
           kH);
  addEdit(hwnd, state->instance, IDC_HOST_EDIT,
          wide(state->config.host).c_str(), xEdit, y, 200, kH);

  y = row(9);
  addLabel(hwnd, state->instance, IDC_PORT_LABEL, L"ポート", xLabel, y, kLabelW,
           kH);
  addEdit(hwnd, state->instance, IDC_PORT_EDIT,
          std::to_wstring(state->config.port).c_str(), xEdit, y, 80, kH,
          ES_NUMBER);

  y = row(10);
  addLabel(hwnd, state->instance, IDC_SERVER_ENCODING_LABEL,
           L"サーバーの文字コード", xLabel, y, kLabelW, kH);
  HWND srvEnc = addCombo(hwnd, state->instance, IDC_SERVER_ENCODING_COMBO,
                         xEdit, y, 160, kH * 5);
  SendMessageW(srvEnc, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(L"euc-jp"));
  SendMessageW(srvEnc, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(L"utf-8"));

  y = row(11);
  addLabel(hwnd, state->instance, IDC_INLINE_CANDIDATES_LABEL,
           L"インライン候補数", xLabel, y, kLabelW, kH);
  addEdit(hwnd, state->instance, IDC_INLINE_CANDIDATES_EDIT,
          std::to_wstring(state->config.inlineCandidates).c_str(), xEdit, y, 60,
          kH, ES_NUMBER);

  y = row(12);
  HWND withoutScope = addButton(hwnd, state->instance, IDC_SKKSERV_WITHOUT_SCOPE_CHECK,
      L"入力欄の属性が取得できない場合もSKKサーバを使う",
      xLabel, y, kClientW - 2 * kMargin, kH, BS_AUTOCHECKBOX);
  SendMessageW(withoutScope, BM_SETCHECK,
      state->config.skkservWithoutInputScope ? BST_CHECKED : BST_UNCHECKED, 0);

  y = row(13);
  addButton(hwnd, state->instance, IDC_SAVE, L"保存", xEdit, y, 80, kH,
            BS_DEFPUSHBUTTON);
  addButton(hwnd, state->instance, IDC_CLOSE, L"閉じる", xEdit + 80 + kGap, y,
            80, kH);

  y = row(14);
  addLabel(hwnd, state->instance, IDC_STATUS, L"", xLabel, y,
           kClientW - xLabel - kMargin, kH * 3);

  SendMessageW(GetDlgItem(hwnd, IDC_ENCODING_COMBO), CB_SETCURSEL,
               (state->config.encoding == "utf-8") ? 1 : 0, 0);
  int srvSel = 0;
  if (state->config.server == "Custom")
    srvSel = 1;
  else if (state->config.server == "Disabled")
    srvSel = 2;
  SendMessageW(GetDlgItem(hwnd, IDC_SKKSERV_COMBO), CB_SETCURSEL, srvSel, 0);
  SendMessageW(GetDlgItem(hwnd, IDC_SERVER_ENCODING_COMBO), CB_SETCURSEL,
               (state->config.serverEncoding == "utf-8") ? 1 : 0, 0);
  SendMessageW(GetDlgItem(hwnd, IDC_AI_CHECK), BM_SETCHECK,
               state->config.ai ? BST_CHECKED : BST_UNCHECKED, 0);

  SendMessageW(GetDlgItem(hwnd, IDC_DICTIONARY_EDIT), EM_SETLIMITTEXT, 32767,
               0);
  SendMessageW(GetDlgItem(hwnd, IDC_USER_DICTIONARY_EDIT), EM_SETLIMITTEXT,
               32767, 0);
  SendMessageW(GetDlgItem(hwnd, IDC_WORKER_EDIT), EM_SETLIMITTEXT, 32767, 0);
  SendMessageW(GetDlgItem(hwnd, IDC_MODEL_EDIT), EM_SETLIMITTEXT, 32767, 0);
  SendMessageW(GetDlgItem(hwnd, IDC_HELPER_EDIT), EM_SETLIMITTEXT, 32767, 0);
  SendMessageW(GetDlgItem(hwnd, IDC_HOST_EDIT), EM_SETLIMITTEXT, 256, 0);
  SendMessageW(GetDlgItem(hwnd, IDC_PORT_EDIT), EM_SETLIMITTEXT, 5, 0);
  SendMessageW(GetDlgItem(hwnd, IDC_INLINE_CANDIDATES_EDIT), EM_SETLIMITTEXT, 2,
               0);
}

LRESULT CALLBACK settingsWndProc(HWND hwnd, UINT msg, WPARAM wParam,
                                 LPARAM lParam) {
  auto *state =
      reinterpret_cast<State *>(GetWindowLongPtrW(hwnd, GWLP_USERDATA));
  try {
    switch (msg) {
    case WM_CREATE: {
      auto *cs = reinterpret_cast<CREATESTRUCTW *>(lParam);
      state =
          new State{cs->hInstance, *static_cast<Config *>(cs->lpCreateParams)};
      SetWindowLongPtrW(hwnd, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(state));

      NONCLIENTMETRICSW ncm{};
      ncm.cbSize = sizeof(ncm);
      if (SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, sizeof(ncm), &ncm, 0))
        state->font = CreateFontIndirectW(&ncm.lfMessageFont);

      createControls(hwnd, state);

      for (HWND child = GetWindow(hwnd, GW_CHILD); child;
           child = GetWindow(child, GW_HWNDNEXT))
        setFont(child, state->font);
      return 0;
    }
    case WM_COMMAND:
      if (HIWORD(wParam) == BN_CLICKED) {
        switch (LOWORD(wParam)) {
        case IDC_DICTIONARY_BROWSE:
          browseOpen(
              hwnd, IDC_DICTIONARY_EDIT,
              L"辞書ファイル\0*.L;*.UTF8;*.TXT\0すべてのファイル\0*.*\0");
          break;
        case IDC_USER_DICTIONARY_BROWSE:
          browseSave(
              hwnd, IDC_USER_DICTIONARY_EDIT,
              L"辞書ファイル\0*.L;*.UTF8;*.TXT\0すべてのファイル\0*.*\0");
          break;
        case IDC_WORKER_BROWSE:
          browseOpen(hwnd, IDC_WORKER_EDIT,
                     L"実行ファイル\0*.exe\0すべてのファイル\0*.*\0");
          break;
        case IDC_MODEL_BROWSE:
          browseOpen(hwnd, IDC_MODEL_EDIT,
                     L"モデルファイル\0*.gguf\0すべてのファイル\0*.*\0");
          break;
        case IDC_HELPER_BROWSE:
          browseOpen(hwnd, IDC_HELPER_EDIT,
                     L"実行ファイル\0*.exe\0すべてのファイル\0*.*\0");
          break;
        case IDOK:
        case IDC_SAVE:
          doSave(hwnd, state);
          break;
        case IDCANCEL:
        case IDC_CLOSE:
          DestroyWindow(hwnd);
          break;
        }
      }
      return 0;
    case WM_SIZE: {
      SCROLLINFO info{};
      info.cbSize = sizeof(info);
      info.fMask = SIF_RANGE | SIF_PAGE;
      info.nMax = row(17);
      info.nPage = HIWORD(lParam);
      SetScrollInfo(hwnd, SB_VERT, &info, TRUE);
      return 0;
    }
    case WM_VSCROLL: {
      SCROLLINFO info{};
      info.cbSize = sizeof(info);
      info.fMask = SIF_ALL;
      GetScrollInfo(hwnd, SB_VERT, &info);
      int next = state->scroll;
      switch (LOWORD(wParam)) {
      case SB_LINEUP:
        next -= kRow;
        break;
      case SB_LINEDOWN:
        next += kRow;
        break;
      case SB_PAGEUP:
        next -= static_cast<int>(info.nPage);
        break;
      case SB_PAGEDOWN:
        next += static_cast<int>(info.nPage);
        break;
      case SB_THUMBTRACK:
        next = info.nTrackPos;
        break;
      }
      next = std::clamp(
          next, 0, std::max(0, info.nMax - static_cast<int>(info.nPage) + 1));
      ScrollWindowEx(hwnd, 0, state->scroll - next, nullptr, nullptr, nullptr,
                     nullptr, SW_SCROLLCHILDREN | SW_INVALIDATE | SW_ERASE);
      state->scroll = next;
      SetScrollPos(hwnd, SB_VERT, next, TRUE);
      return 0;
    }
    case WM_MOUSEWHEEL:
      SendMessageW(hwnd, WM_VSCROLL,
                   GET_WHEEL_DELTA_WPARAM(wParam) > 0 ? SB_LINEUP : SB_LINEDOWN,
                   0);
      return 0;
    case WM_CLOSE:
      DestroyWindow(hwnd);
      return 0;
    case WM_DESTROY:
      PostQuitMessage(0);
      return 0;
    case WM_NCDESTROY:
      if (state) {
        if (state->font)
          DeleteObject(state->font);
        delete state;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
      }
      break;
    }
    return DefWindowProcW(hwnd, msg, wParam, lParam);
  } catch (...) {
    if (msg == WM_CREATE)
      return -1;
    SetWindowTextW(GetDlgItem(hwnd, IDC_STATUS),
                   L"設定画面の操作に失敗しました。");
    return 0;
  }
}

} // namespace

HWND createSettingsWindow(HINSTANCE instance, const Config &config, bool show) {
  static bool registered = false;
  if (!registered) {
    WNDCLASSEXW wc{};
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = settingsWndProc;
    wc.hInstance = instance;
    wc.hCursor = LoadCursorW(nullptr, IDC_ARROW);
    wc.hbrBackground = reinterpret_cast<HBRUSH>(COLOR_BTNFACE + 1);
    wc.lpszClassName = L"tgskk-settings";
    if (!RegisterClassExW(&wc))
      return nullptr;
    registered = true;
  }

  RECT rc{0, 0, kClientW, row(16)};
  AdjustWindowRectEx(&rc,
                     WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX |
                         WS_CLIPCHILDREN | WS_VSCROLL,
                     FALSE, WS_EX_CONTROLPARENT);

  RECT work{};
  SystemParametersInfoW(SPI_GETWORKAREA, 0, &work, 0);
  const auto height =
      work.bottom > work.top
          ? std::min(rc.bottom - rc.top, work.bottom - work.top - 24)
          : rc.bottom - rc.top;
  HWND hwnd = CreateWindowExW(
      WS_EX_CONTROLPARENT, L"tgskk-settings", L"tgskk 設定",
      WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX |
          WS_CLIPCHILDREN | WS_VSCROLL,
      CW_USEDEFAULT, CW_USEDEFAULT, rc.right - rc.left, height, nullptr,
      nullptr, instance, const_cast<Config *>(&config));
  if (!hwnd)
    return nullptr;
  if (show)
    ShowWindow(hwnd, SW_SHOW);
  return hwnd;
}

} // namespace tgskk::windows
