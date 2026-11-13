// SPDX-License-Identifier: GPL-3.0-or-later
#include "tgskk.h"
#include "tsf_compat.h"
#include <atomic>
#include <filesystem>
#include <inputscope.h>
#include <memory>
#include <msctf.h>
#include <shlobj.h>
#include <string>
#include <unordered_map>
#include <vector>
#include <windows.h>

namespace {
const CLSID serviceId = {0xc7993bd4,
                         0x1094,
                         0x4968,
                         {0xa3, 0x11, 0x3d, 0xb8, 0xa0, 0xe0, 0x17, 0x39}};
const GUID profileId = {0x70c846ab,
                        0x9fb5,
                        0x43cb,
                        {0x94, 0xbb, 0x74, 0x83, 0xb9, 0xb2, 0xf6, 0xf8}};
HMODULE module;
std::atomic<long> objects = 0;
std::atomic<long> locks = 0;

std::wstring wide(const char *text) {
  if (!text || !*text)
    return {};
  int n =
      MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text, -1, nullptr, 0);
  if (!n)
    return {};
  std::wstring out(n, L'\0');
  MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text, -1, out.data(), n);
  out.pop_back();
  return out;
}
std::string utf8(const std::wstring &text) {
  int n = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text.data(),
                              static_cast<int>(text.size()), nullptr, 0,
                              nullptr, nullptr);
  std::string out(n, '\0');
  if (n)
    WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text.data(),
                        static_cast<int>(text.size()), out.data(), n, nullptr,
                        nullptr);
  return out;
}
std::wstring modulePath() {
  std::vector<wchar_t> buffer(32768);
  DWORD n = GetModuleFileNameW(module, buffer.data(),
                               static_cast<DWORD>(buffer.size()));
  if (!n || n == buffer.size())
    return {};
  return {buffer.data(), n};
}
template <class T> void release(T *&value) {
  if (value) {
    value->Release();
    value = nullptr;
  }
}
class Cleanup final : public ITfEditSession {
  std::atomic<ULONG> refs{1};
  ITfComposition *composition;
  std::shared_ptr<bool> live;
  bool erase;

public:
  Cleanup(ITfComposition *value, std::shared_ptr<bool> token, bool clear)
      : composition(value), live(std::move(token)), erase(clear) {
    composition->AddRef();
    ++objects;
  }
  ~Cleanup() {
    composition->Release();
    --objects;
  }
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void **out) override {
    if (!out)
      return E_POINTER;
    *out = nullptr;
    if (id != IID_IUnknown && id != IID_ITfEditSession)
      return E_NOINTERFACE;
    *out = static_cast<ITfEditSession *>(this);
    AddRef();
    return S_OK;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++refs; }
  ULONG STDMETHODCALLTYPE Release() override {
    auto n = --refs;
    if (!n)
      delete this;
    return n;
  }
  HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie cookie) override {
    if (!*live)
      return S_OK;
    ITfRange *range = nullptr;
    HRESULT result = S_OK;
    if (erase) {
      result = composition->GetRange(&range);
      if (SUCCEEDED(result)) {
        result = range->SetText(cookie, 0, L"", 0);
        release(range);
      }
    }
    if (FAILED(result))
      return result;
    result = composition->EndComposition(cookie);
    if (SUCCEEDED(result))
      *live = false;
    return result;
  }
};
class Service;
class Edit final : public ITfEditSession {
  std::atomic<ULONG> refs{1};
  Service *owner;
  ITfContext *context;
  uint32_t symbol, modifiers;
  bool reset;

public:
  bool eaten = false;
  Edit(Service *, ITfContext *, uint32_t, uint32_t, bool = false);
  ~Edit();
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **out) override;
  ULONG STDMETHODCALLTYPE AddRef() override { return ++refs; }
  ULONG STDMETHODCALLTYPE Release() override {
    auto n = --refs;
    if (!n)
      delete this;
    return n;
  }
  HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie) override;
};
class Service final : public ITfTextInputProcessorEx,
                      public ITfKeyEventSink,
                      public ITfCompositionSink,
                      public ITfThreadMgrEventSink,
                      public ITfTextEditSink,
                      public ITfThreadFocusSink {
  std::atomic<ULONG> refs{1};
  ITfThreadMgr *manager = nullptr;
  TfClientId client = TF_CLIENTID_NULL;
  ITfContext *active = nullptr;
  ITfComposition *composition = nullptr;
  DWORD threadCookie = TF_INVALID_COOKIE;
  DWORD editCookie = TF_INVALID_COOKIE;
  DWORD focusCookie = TF_INVALID_COOKIE;
  std::unordered_map<ITfComposition *, std::shared_ptr<bool>> lifetimes;
  void unwatch() {
    if (active && editCookie != TF_INVALID_COOKIE) {
      ITfSource *source = nullptr;
      if (SUCCEEDED(active->QueryInterface(
              IID_ITfSource, reinterpret_cast<void **>(&source)))) {
        source->UnadviseSink(editCookie);
        release(source);
      }
    }
    editCookie = TF_INVALID_COOKIE;
  }
  void abandon(bool preserve = false) {
    if (composition && active) {
      auto found = lifetimes.find(composition);
      if (found != lifetimes.end()) {
        auto *cleanup = new Cleanup(composition, found->second, !preserve);
        HRESULT status = E_FAIL;
        active->RequestEditSession(client, cleanup,
                                   TF_ES_ASYNC | TF_ES_READWRITE, &status);
        cleanup->Release();
      }
    }
    release(composition);
    clearCore();
  }
  TgskkRuntime *runtime = nullptr;
  TgskkSession *session = nullptr;
  HWND candidates = nullptr;
  HRESULT finish(TfEditCookie cookie) {
    if (!composition)
      return S_OK;
    ITfComposition *current = composition;
    current->AddRef();
    HRESULT result = current->EndComposition(cookie);
    if (SUCCEEDED(result)) {
      auto found = lifetimes.find(current);
      if (found != lifetimes.end()) {
        *found->second = false;
        lifetimes.erase(found);
      }
      if (composition == current)
        release(composition);
    }
    current->Release();
    return result;
  }
  bool enabled = true;
  static bool modifier(WPARAM key) {
    return key == VK_SHIFT || key == VK_CONTROL || key == VK_MENU ||
           key == VK_LWIN || key == VK_RWIN || key == VK_CAPITAL ||
           key == VK_NUMLOCK || key == VK_SCROLL ||
           (key >= VK_LSHIFT && key <= VK_RMENU);
  }
  bool editing = false;
  void clearCore() {
    if (session)
      tgskk_session_reset(session);
    hideCandidates();
  }
  void hideCandidates() {
    if (candidates)
      ShowWindow(candidates, SW_HIDE);
  }
  bool disabled(ITfContext *context) {
    ITfCompartmentMgr *compartments = nullptr;
    bool result = false;
    if (SUCCEEDED(context->QueryInterface(
            IID_ITfCompartmentMgr, reinterpret_cast<void **>(&compartments)))) {
      for (auto id : {GUID_COMPARTMENT_KEYBOARD_DISABLED,
                      GUID_COMPARTMENT_EMPTYCONTEXT}) {
        ITfCompartment *value = nullptr;
        if (SUCCEEDED(compartments->GetCompartment(id, &value))) {
          VARIANT data;
          VariantInit(&data);
          if (SUCCEEDED(value->GetValue(&data)) && data.vt == VT_I4 &&
              data.lVal)
            result = true;
          VariantClear(&data);
          release(value);
        }
      }
      release(compartments);
    }
    return result;
  }
  bool translate(WPARAM key, LPARAM scan, uint32_t &symbol,
                 uint32_t &modifiers) {
    if (key == VK_SHIFT || key == VK_CONTROL || key == VK_MENU ||
        key == VK_LWIN || key == VK_RWIN)
      return false;
    BYTE state[256];
    if (!GetKeyboardState(state))
      return false;
    modifiers = ((state[VK_SHIFT] & 0x80) ? 1 : 0) |
                ((state[VK_CONTROL] & 0x80) ? 4 : 0) |
                ((state[VK_MENU] & 0x80) ? 8 : 0);
    if ((modifiers & 8) || (state[VK_LWIN] & 0x80) || (state[VK_RWIN] & 0x80))
      return false;
    switch (key) {
    case VK_RETURN:
      symbol = 0xff0d;
      break;
    case VK_BACK:
      symbol = 0xff08;
      break;
    case VK_ESCAPE:
      symbol = 0xff1b;
      break;
    case VK_TAB:
      symbol = 0xff09;
      break;
    case VK_LEFT:
      symbol = 0xff51;
      break;
    case VK_UP:
      symbol = 0xff52;
      break;
    case VK_RIGHT:
      symbol = 0xff53;
      break;
    case VK_DOWN:
      symbol = 0xff54;
      break;
    case VK_PRIOR:
      symbol = 0xff55;
      break;
    case VK_NEXT:
      symbol = 0xff56;
      break;
    default: {
      state[VK_CONTROL] = state[VK_LCONTROL] = state[VK_RCONTROL] = 0;
      wchar_t text[8];
      int n = ToUnicodeEx(static_cast<UINT>(key),
                          (static_cast<UINT>(scan) >> 16) & 0xff, state, text,
                          8, 4, GetKeyboardLayout(0));
      if (n != 1 || text[0] < 0x20 || text[0] > 0x7e)
        return false;
      symbol = text[0];
      if ((modifiers & 4) && !(modifiers & 1) && symbol >= 'A' && symbol <= 'Z')
        symbol += 'a' - 'A';
    }
    }
    return true;
  }
  HRESULT request(ITfContext *context, uint32_t symbol, uint32_t modifiers,
                  BOOL *eaten, bool reset = false) {
    auto *edit = new Edit(this, context, symbol, modifiers, reset);
    HRESULT result = E_FAIL;
    HRESULT call = context->RequestEditSession(
        client, edit, TF_ES_SYNC | TF_ES_READWRITE, &result);
    if (eaten)
      *eaten = SUCCEEDED(call) && edit->eaten;
    edit->Release();
    return FAILED(call) ? call : result;
  }

public:
  Service() { ++objects; }
  ~Service() {
    Deactivate();
    --objects;
  }
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **out) override {
    if (!out)
      return E_POINTER;
    *out = nullptr;
    if (iid == IID_IUnknown || iid == IID_ITfTextInputProcessor ||
        iid == IID_ITfTextInputProcessorEx)
      *out = static_cast<ITfTextInputProcessorEx *>(this);
    else if (iid == IID_ITfKeyEventSink)
      *out = static_cast<ITfKeyEventSink *>(this);
    else if (iid == IID_ITfCompositionSink)
      *out = static_cast<ITfCompositionSink *>(this);
    else if (iid == IID_ITfThreadFocusSink)
      *out = static_cast<ITfThreadFocusSink *>(this);
    else if (iid == IID_ITfTextEditSink)
      *out = static_cast<ITfTextEditSink *>(this);
    else if (iid == IID_ITfThreadMgrEventSink)
      *out = static_cast<ITfThreadMgrEventSink *>(this);
    if (!*out)
      return E_NOINTERFACE;
    AddRef();
    return S_OK;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++refs; }
  ULONG STDMETHODCALLTYPE Release() override {
    auto n = --refs;
    if (!n)
      delete this;
    return n;
  }
  HRESULT STDMETHODCALLTYPE Activate(ITfThreadMgr *value,
                                     TfClientId id) override {
    return ActivateEx(value, id, 0);
  }
  HRESULT STDMETHODCALLTYPE ActivateEx(ITfThreadMgr *value, TfClientId id,
                                       DWORD flags) override {
    if (manager)
      return E_UNEXPECTED;
    if (flags & 6)
      return E_ACCESSDENIED;
    std::wstring path =
        (std::filesystem::path(modulePath()).parent_path() / L"SKK-JISYO.L")
            .wstring();
    wchar_t configured[32768];
    DWORD n = GetEnvironmentVariableW(L"TGSKK_DICTIONARY", configured, 32768);
    if (n && n < 32768)
      path.assign(configured, n);
    char *error = nullptr;
    runtime = tgskk_runtime_new(utf8(path).c_str(), "euc-jp", nullptr, &error);
    tgskk_string_free(error);
    if (!runtime)
      return E_FAIL;
    session = tgskk_session_new(runtime, &error);
    tgskk_string_free(error);
    if (!session) {
      tgskk_runtime_free(runtime);
      runtime = nullptr;
      return E_FAIL;
    }
    manager = value;
    manager->AddRef();
    client = id;
    ITfKeystrokeMgr *keys = nullptr;
    HRESULT result = manager->QueryInterface(IID_ITfKeystrokeMgr,
                                             reinterpret_cast<void **>(&keys));
    if (SUCCEEDED(result)) {
      result = keys->AdviseKeyEventSink(client, this, TRUE);
      release(keys);
    }
    if (FAILED(result)) {
      Deactivate();
      return result;
    }
    ITfSource *source = nullptr;
    result = manager->QueryInterface(IID_ITfSource,
                                     reinterpret_cast<void **>(&source));
    if (SUCCEEDED(result)) {
      result = source->AdviseSink(IID_ITfThreadMgrEventSink,
                                  static_cast<ITfThreadMgrEventSink *>(this),
                                  &threadCookie);
      if (SUCCEEDED(result))
        result = source->AdviseSink(IID_ITfThreadFocusSink,
                                    static_cast<ITfThreadFocusSink *>(this),
                                    &focusCookie);
      release(source);
    }
    if (FAILED(result)) {
      Deactivate();
      return result;
    }
    candidates =
        CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                        L"STATIC", L"", WS_POPUP | WS_BORDER | SS_LEFT, 0, 0,
                        300, 200, nullptr, nullptr, module, nullptr);
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE Deactivate() override {
    abandon();
    unwatch();
    release(active);
    if (manager) {
      ITfKeystrokeMgr *keys = nullptr;
      if (SUCCEEDED(manager->QueryInterface(
              IID_ITfKeystrokeMgr, reinterpret_cast<void **>(&keys)))) {
        keys->UnadviseKeyEventSink(client);
        release(keys);
      }
      ITfSource *source = nullptr;
      if (threadCookie != TF_INVALID_COOKIE &&
          SUCCEEDED(manager->QueryInterface(
              IID_ITfSource, reinterpret_cast<void **>(&source)))) {
        source->UnadviseSink(threadCookie);
        if (focusCookie != TF_INVALID_COOKIE)
          source->UnadviseSink(focusCookie);
        release(source);
      }
    }
    threadCookie = TF_INVALID_COOKIE;
    focusCookie = TF_INVALID_COOKIE;
    release(manager);
    client = TF_CLIENTID_NULL;
    if (candidates)
      DestroyWindow(candidates);
    candidates = nullptr;
    if (session)
      tgskk_session_free(session);
    session = nullptr;
    if (runtime)
      tgskk_runtime_free(runtime);
    runtime = nullptr;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnTestKeyDown(ITfContext *context, WPARAM key,
                                          LPARAM scan, BOOL *eaten) override {
    if (!eaten)
      return E_POINTER;
    *eaten = FALSE;
    if (modifier(key))
      return S_OK;
    uint32_t symbol, modifiers;
    if (composition && enabled && session && !disabled(context)) {
      *eaten = TRUE;
      return S_OK;
    }
    if (enabled && session && !disabled(context) &&
        translate(key, scan, symbol, modifiers))
      *eaten = tgskk_session_may_process(session, symbol, modifiers) > 0;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnKeyDown(ITfContext *context, WPARAM key,
                                      LPARAM scan, BOOL *eaten) override {
    if (!eaten)
      return E_POINTER;
    *eaten = FALSE;
    if (modifier(key))
      return S_OK;
    uint32_t symbol, modifiers;
    if (!enabled || !session || disabled(context) ||
        !translate(key, scan, symbol, modifiers)) {
      abandon();
      return S_OK;
    }
    if (active != context) {
      abandon();
      unwatch();
      release(active);
      active = context;
      active->AddRef();
      ITfSource *source = nullptr;
      if (SUCCEEDED(active->QueryInterface(
              IID_ITfSource, reinterpret_cast<void **>(&source)))) {
        source->AdviseSink(IID_ITfTextEditSink,
                           static_cast<ITfTextEditSink *>(this), &editCookie);
        release(source);
      }
    }
    HRESULT result = request(context, symbol, modifiers, eaten);
    if (FAILED(result))
      abandon(*eaten != FALSE);
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnTestKeyUp(ITfContext *, WPARAM, LPARAM,
                                        BOOL *eaten) override {
    if (!eaten)
      return E_POINTER;
    *eaten = FALSE;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnKeyUp(ITfContext *, WPARAM, LPARAM,
                                    BOOL *eaten) override {
    if (!eaten)
      return E_POINTER;
    *eaten = FALSE;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnPreservedKey(ITfContext *, REFGUID,
                                           BOOL *eaten) override {
    if (!eaten)
      return E_POINTER;
    *eaten = FALSE;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnSetFocus(BOOL foreground) override {
    if (!foreground)
      abandon();
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE
  OnCompositionTerminated(TfEditCookie, ITfComposition *value) override {
    auto found = lifetimes.find(value);
    if (found != lifetimes.end()) {
      *found->second = false;
      lifetimes.erase(found);
    }
    if (!editing && value == composition) {
      release(composition);
      clearCore();
    }
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnSetThreadFocus() override { return S_OK; }
  HRESULT STDMETHODCALLTYPE OnKillThreadFocus() override {
    abandon();
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnInitDocumentMgr(ITfDocumentMgr *) override {
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnUninitDocumentMgr(ITfDocumentMgr *) override {
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnPushContext(ITfContext *) override {
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnPopContext(ITfContext *context) override {
    if (active == context) {
      abandon();
      unwatch();
      release(active);
    }
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnSetFocus(ITfDocumentMgr *,
                                       ITfDocumentMgr *) override {
    abandon();
    unwatch();
    release(active);
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnEndEdit(ITfContext *context, TfEditCookie cookie,
                                      ITfEditRecord *record) override {
    if (editing || context != active || !composition)
      return S_OK;
    BOOL changed = FALSE;
    if (FAILED(record->GetSelectionStatus(&changed)) || !changed)
      return S_OK;
    TF_SELECTION selection{};
    ULONG count = 0;
    ITfRange *range = nullptr;
    if (SUCCEEDED(context->GetSelection(cookie, TF_DEFAULT_SELECTION, 1,
                                        &selection, &count)) &&
        count == 1 && SUCCEEDED(composition->GetRange(&range))) {
      LONG start = 1, end = 1;
      selection.range->CompareStart(cookie, range, TF_ANCHOR_END, &start);
      selection.range->CompareEnd(cookie, range, TF_ANCHOR_END, &end);
      if (start != 0 || end != 0)
        abandon();
    }
    release(range);
    release(selection.range);
    return S_OK;
  }
  HRESULT edit(ITfContext *context, TfEditCookie cookie, uint32_t symbol,
               uint32_t modifiers, bool reset, bool &eaten) {
    eaten = false;
    if (!session || (active && context != active))
      return TF_E_DISCONNECTED;
    ITfRange *range = nullptr;
    bool fresh = !composition;
    if (composition)
      composition->GetRange(&range);
    if (!range) {
      TF_SELECTION selection{};
      ULONG fetched = 0;
      HRESULT hr = context->GetSelection(cookie, TF_DEFAULT_SELECTION, 1,
                                         &selection, &fetched);
      if (FAILED(hr) || fetched != 1 || !selection.range)
        return E_FAIL;
      range = selection.range;
    }
    if (!reset) {
      ITfReadOnlyProperty *property = nullptr;
      HRESULT scopeResult =
          context->GetAppProperty(GUID_PROP_INPUTSCOPE, &property);
      if (SUCCEEDED(scopeResult) && property) {
        VARIANT value;
        VariantInit(&value);
        scopeResult = property->GetValue(cookie, range, &value);
        bool sensitive = false;
        if (SUCCEEDED(scopeResult) && value.vt == VT_UNKNOWN && value.punkVal) {
          ITfInputScope *scope = nullptr;
          scopeResult = value.punkVal->QueryInterface(
              IID_ITfInputScope, reinterpret_cast<void **>(&scope));
          if (SUCCEEDED(scopeResult)) {
            InputScope *scopes = nullptr;
            UINT count = 0;
            scopeResult = scope->GetInputScopes(&scopes, &count);
            if (SUCCEEDED(scopeResult))
              for (UINT i = 0; i < count; ++i)
                if (scopes[i] == IS_PASSWORD || scopes[i] == IS_PRIVATE)
                  sensitive = true;
            CoTaskMemFree(scopes);
            release(scope);
          }
        }
        VariantClear(&value);
        release(property);
        if (sensitive || FAILED(scopeResult)) {
          release(range);
          return E_ACCESSDENIED;
        }
      }
    }
    if (reset && fresh) {
      clearCore();
      release(range);
      return S_OK;
    }
    std::erase_if(lifetimes, [](const auto &entry) { return !*entry.second; });
    editing = true;
    if (fresh) {
      ITfContextComposition *owner = nullptr;
      HRESULT hr = context->QueryInterface(IID_ITfContextComposition,
                                           reinterpret_cast<void **>(&owner));
      if (SUCCEEDED(hr)) {
        hr = owner->StartComposition(cookie, range, this, &composition);
        release(owner);
      }
      if (FAILED(hr) || !composition) {
        editing = false;
        release(range);
        return FAILED(hr) ? hr : E_FAIL;
      }
      lifetimes[composition] = std::make_shared<bool>(true);
    }
    const TgskkView *view =
        reset ? tgskk_session_reset(session)
              : tgskk_session_key(session, symbol, modifiers, 0);
    if (!view) {
      editing = false;
      release(range);
      return E_FAIL;
    }
    bool consumed = view->consumed;
    if (!consumed && !*view->committed && !reset) {
      reset = true;
      view = tgskk_session_reset(session);
      if (!view) {
        editing = false;
        release(range);
        return E_FAIL;
      }
    }
    std::wstring committed = wide(view->committed),
                 preedit = wide(view->preedit);
    HRESULT result = S_OK;
    if (reset) {
      if (!fresh)
        result = range->SetText(cookie, 0, L"", 0);
      if (SUCCEEDED(result)) {
        result = finish(cookie);
      }
      hideCandidates();
    } else {
      std::wstring combined = committed + preedit;
      bool changed = !combined.empty() || !fresh;
      if (changed)
        result = range->SetText(cookie, 0, combined.data(),
                                static_cast<LONG>(combined.size()));
      if (SUCCEEDED(result))
        eaten = consumed;
      if (SUCCEEDED(result) && preedit.empty()) {
        result = finish(cookie);
      } else if (SUCCEEDED(result) && !committed.empty()) {
        ITfRange *tail = nullptr;
        result = range->Clone(&tail);
        if (SUCCEEDED(result)) {
          LONG shifted = 0;
          result = tail->ShiftStart(cookie, static_cast<LONG>(committed.size()),
                                    &shifted, nullptr);
          if (SUCCEEDED(result))
            result = composition->ShiftStart(cookie, tail);
          release(tail);
        }
      }
      if (SUCCEEDED(result) && changed) {
        range->Collapse(cookie, TF_ANCHOR_END);
        TF_SELECTION selection{range, {TF_AE_NONE, FALSE}};
        result = context->SetSelection(cookie, 1, &selection);
      }
      hideCandidates();
      if (SUCCEEDED(result) && candidates && view->selecting &&
          view->selected >= 4) {
        std::wstring text;
        size_t start = 4 + ((view->selected - 4) / 7) * 7;
        for (size_t i = start; i < view->candidate_count && i < start + 7; ++i)
          text += (i == view->selected ? L"> " : L"  ") +
                  wide(tgskk_candidate_text(session, i)) + L"\r\n";
        ITfContextView *viewport = nullptr;
        if (SUCCEEDED(context->GetActiveView(&viewport))) {
          RECT caret{};
          BOOL clipped = FALSE;
          if (SUCCEEDED(
                  viewport->GetTextExt(cookie, range, &caret, &clipped))) {
            SetWindowTextW(candidates, text.c_str());
            SetWindowPos(candidates, HWND_TOPMOST, caret.left, caret.bottom,
                         320,
                         static_cast<int>((view->candidate_count - start < 7
                                               ? view->candidate_count - start
                                               : 7) *
                                              24 +
                                          8),
                         SWP_NOACTIVATE | SWP_SHOWWINDOW);
          }
          release(viewport);
        }
      }
    }
    editing = false;
    release(range);
    return result;
  }
};
Edit::Edit(Service *value, ITfContext *ctx, uint32_t key, uint32_t mods,
           bool clear)
    : owner(value), context(ctx), symbol(key), modifiers(mods), reset(clear) {
  owner->AddRef();
  context->AddRef();
}
Edit::~Edit() {
  context->Release();
  owner->Release();
}
HRESULT Edit::QueryInterface(REFIID iid, void **out) {
  if (!out)
    return E_POINTER;
  *out = nullptr;
  if (iid != IID_IUnknown && iid != IID_ITfEditSession)
    return E_NOINTERFACE;
  *out = static_cast<ITfEditSession *>(this);
  AddRef();
  return S_OK;
}
HRESULT Edit::DoEditSession(TfEditCookie cookie) {
  return owner->edit(context, cookie, symbol, modifiers, reset, eaten);
}

class Factory final : public IClassFactory {
  std::atomic<ULONG> refs{1};

public:
  Factory() { ++objects; }
  ~Factory() { --objects; }
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **out) override {
    if (!out)
      return E_POINTER;
    *out = nullptr;
    if (iid != IID_IUnknown && iid != IID_IClassFactory)
      return E_NOINTERFACE;
    *out = static_cast<IClassFactory *>(this);
    AddRef();
    return S_OK;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++refs; }
  ULONG STDMETHODCALLTYPE Release() override {
    auto n = --refs;
    if (!n)
      delete this;
    return n;
  }
  HRESULT STDMETHODCALLTYPE CreateInstance(IUnknown *outer, REFIID iid,
                                           void **out) override {
    if (outer)
      return CLASS_E_NOAGGREGATION;
    auto *service = new Service;
    HRESULT result = service->QueryInterface(iid, out);
    service->Release();
    return result;
  }
  HRESULT STDMETHODCALLTYPE LockServer(BOOL lock) override {
    if (lock)
      ++locks;
    else
      --locks;
    return S_OK;
  }
};
HRESULT registry(bool install) {
  wchar_t guid[64];
  StringFromGUID2(serviceId, guid, 64);
  std::wstring key = std::wstring(L"Software\\Classes\\CLSID\\") + guid;
  if (!install) {
    auto status = RegDeleteTreeW(HKEY_LOCAL_MACHINE, key.c_str());
    return status == ERROR_FILE_NOT_FOUND || status == ERROR_SUCCESS
               ? S_OK
               : HRESULT_FROM_WIN32(status);
  }
  key += L"\\InprocServer32";
  HKEY handle;
  auto status = RegCreateKeyExW(HKEY_LOCAL_MACHINE, key.c_str(), 0, nullptr, 0,
                                KEY_WRITE, nullptr, &handle, nullptr);
  if (status != ERROR_SUCCESS)
    return HRESULT_FROM_WIN32(status);
  auto path = modulePath();
  status = RegSetValueExW(
      handle, nullptr, 0, REG_SZ, reinterpret_cast<const BYTE *>(path.c_str()),
      static_cast<DWORD>((path.size() + 1) * sizeof(wchar_t)));
  if (status == ERROR_SUCCESS)
    status = RegSetValueExW(handle, L"ThreadingModel", 0, REG_SZ,
                            reinterpret_cast<const BYTE *>(L"Apartment"),
                            sizeof(L"Apartment"));
  RegCloseKey(handle);
  return HRESULT_FROM_WIN32(status);
}
} // namespace
extern "C" BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID) {
  if (reason == DLL_PROCESS_ATTACH) {
    module = instance;
    DisableThreadLibraryCalls(instance);
  }
  return TRUE;
}
extern "C" HRESULT __stdcall DllCanUnloadNow() {
  return objects == 0 && locks == 0 ? S_OK : S_FALSE;
}
extern "C" HRESULT __stdcall DllGetClassObject(REFCLSID id, REFIID iid,
                                               void **out) {
  if (id != serviceId)
    return CLASS_E_CLASSNOTAVAILABLE;
  auto *factory = new Factory;
  auto result = factory->QueryInterface(iid, out);
  factory->Release();
  return result;
}
extern "C" HRESULT __stdcall DllRegisterServer() {
  HRESULT initialized = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
  if (FAILED(initialized) && initialized != RPC_E_CHANGED_MODE)
    return initialized;
  HRESULT result = registry(true);
  ITfInputProcessorProfiles *profiles = nullptr;
  if (SUCCEEDED(result))
    result = CoCreateInstance(
        CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
        IID_ITfInputProcessorProfiles, reinterpret_cast<void **>(&profiles));
  if (SUCCEEDED(result))
    result = profiles->Register(serviceId);
  if (SUCCEEDED(result)) {
    auto path = modulePath();
    result = profiles->AddLanguageProfile(
        serviceId, MAKELANGID(LANG_JAPANESE, SUBLANG_DEFAULT), profileId,
        L"tgskk", 5, path.c_str(), static_cast<ULONG>(path.size()), 0);
  }
  ITfCategoryMgr *categories = nullptr;
  if (SUCCEEDED(result))
    result = CoCreateInstance(CLSID_TF_CategoryMgr, nullptr,
                              CLSCTX_INPROC_SERVER, IID_ITfCategoryMgr,
                              reinterpret_cast<void **>(&categories));
  if (SUCCEEDED(result))
    result = categories->RegisterCategory(serviceId, GUID_TFCAT_TIP_KEYBOARD,
                                          serviceId);
  release(categories);
  release(profiles);
  if (SUCCEEDED(initialized))
    CoUninitialize();
  return result;
}
extern "C" HRESULT __stdcall DllUnregisterServer() {
  HRESULT initialized = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
  if (FAILED(initialized) && initialized != RPC_E_CHANGED_MODE)
    return initialized;
  ITfInputProcessorProfiles *profiles = nullptr;
  if (SUCCEEDED(CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr,
                                 CLSCTX_INPROC_SERVER,
                                 IID_ITfInputProcessorProfiles,
                                 reinterpret_cast<void **>(&profiles)))) {
    profiles->Unregister(serviceId);
    release(profiles);
  }
  ITfCategoryMgr *categories = nullptr;
  if (SUCCEEDED(CoCreateInstance(CLSID_TF_CategoryMgr, nullptr,
                                 CLSCTX_INPROC_SERVER, IID_ITfCategoryMgr,
                                 reinterpret_cast<void **>(&categories)))) {
    categories->UnregisterCategory(serviceId, GUID_TFCAT_TIP_KEYBOARD,
                                   serviceId);
    release(categories);
  }
  HRESULT result = registry(false);
  if (SUCCEEDED(initialized))
    CoUninitialize();
  return result;
}
