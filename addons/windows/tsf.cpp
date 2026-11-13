// SPDX-License-Identifier: GPL-3.0-or-later
#include "../common/candidates.h"
#include "broker_client.h"
#include "tgskk.h"
#include "tsf_compat.h"
#include <algorithm>
#include <atomic>
#include <cstdio>
#include <filesystem>
#include <inputscope.h>
#include <memory>
#include <msctf.h>
#include <new>
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
bool debugEnabled() {
  static const bool enabled = [] {
    wchar_t value[2];
    return GetEnvironmentVariableW(L"TGSKK_DEBUG", value, 2) != 0;
  }();
  return enabled;
}
void traceCount(const char *operation, long count,
                const char *kind = "objects") {
  if (debugEnabled())
    std::fprintf(stderr, "tgskk: %s %s=%ld\n", operation, kind, count);
}
HRESULT reportFailure(const char *stage, HRESULT result) {
  char message[160];
  std::snprintf(message, sizeof(message), "tgskk: %s failed (%08lx)\n", stage,
                static_cast<unsigned long>(result));
  OutputDebugStringA(message);
  if (debugEnabled())
    std::fputs(message, stderr);
  return result;
}
void reportCoreFailure(const char *stage, const char *error) {
  reportFailure(stage, E_FAIL);
  if (error && *error) {
    OutputDebugStringA(error);
    OutputDebugStringA("\n");
    if (debugEnabled())
      std::fprintf(stderr, "tgskk: %s\n", error);
  }
}

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
struct CompositionLife {
  bool live = true;
  bool cleanupQueued = false;
  bool erase = true;
  std::wstring preedit;
};
HRESULT eraseComposition(ITfComposition *composition, TfEditCookie cookie,
                         const std::wstring &expected) {
  std::vector<wchar_t> text;
  try {
    text.resize(expected.size() + 1);
  } catch (...) {
    return E_OUTOFMEMORY;
  }
  ITfRange *range = nullptr;
  ITfContext *context = nullptr;
  TF_SELECTION selection{};
  ULONG fetched = 0, length = 0;
  HRESULT result = composition->GetRange(&range);
  if (SUCCEEDED(result))
    result = range->GetText(cookie, 0, text.data(),
                            static_cast<ULONG>(text.size()), &length);
  if (SUCCEEDED(result) &&
      (length != expected.size() ||
       !std::equal(expected.begin(), expected.end(), text.begin()))) {
    release(range);
    return S_FALSE;
  }
  if (SUCCEEDED(result))
    result = range->GetContext(&context);
  if (SUCCEEDED(result))
    result = context->GetSelection(cookie, TF_DEFAULT_SELECTION, 1, &selection,
                                   &fetched);
  if (SUCCEEDED(result) && (fetched != 1 || !selection.range))
    result = E_FAIL;
  if (SUCCEEDED(result))
    result = range->SetText(cookie, 0, L"", 0);
  if (SUCCEEDED(result))
    result = context->SetSelection(cookie, 1, &selection);
  release(selection.range);
  release(context);
  release(range);
  return result;
}
class Cleanup final : public ITfEditSession {
  std::atomic<ULONG> refs{1};
  ITfComposition *composition;
  std::shared_ptr<CompositionLife> life;

public:
  Cleanup(ITfComposition *value, std::shared_ptr<CompositionLife> token,
          bool clear)
      : composition(value), life(std::move(token)) {
    life->erase &= clear;
    composition->AddRef();
    traceCount("cleanup created", ++objects);
  }
  ~Cleanup() {
    composition->Release();
    traceCount("cleanup destroyed", --objects);
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
    life->cleanupQueued = false;
    if (!life->live)
      return S_OK;
    HRESULT result = life->erase
                         ? eraseComposition(composition, cookie, life->preedit)
                         : S_OK;
    const HRESULT ended = composition->EndComposition(cookie);
    if (SUCCEEDED(ended))
      life->live = false;
    result = FAILED(result) ? result : ended;
    return FAILED(result) ? reportFailure("end composition", result) : result;
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
  std::unordered_map<ITfComposition *, std::shared_ptr<CompositionLife>>
      lifetimes;
  void unwatch() {
    if (active && editCookie != TF_INVALID_COOKIE) {
      ITfSource *source = nullptr;
      if (SUCCEEDED(active->QueryInterface(
              IID_ITfSource, reinterpret_cast<void **>(&source)))) {
        const HRESULT result = source->UnadviseSink(editCookie);
        if (FAILED(result))
          reportFailure("unwatch context", result);
        release(source);
      }
    }
    editCookie = TF_INVALID_COOKIE;
  }
  bool cancelPending = false;
  bool abandon(bool preserve = false) {
    if (composition && active) {
      auto found = lifetimes.find(composition);
      if (found == lifetimes.end()) {
        cancelPending = true;
        clearCore();
        return false;
      }
      const auto life = found->second;
      life->erase &= !preserve;
      if (!life->live) {
        lifetimes.erase(found);
        release(composition);
        cancelPending = false;
        clearCore();
        return true;
      }
      if (life->cleanupQueued) {
        cancelPending = true;
        clearCore();
        return false;
      }
      auto *cleanup =
          new (std::nothrow) Cleanup(composition, found->second, !preserve);
      if (!cleanup) {
        cancelPending = true;
        clearCore();
        return false;
      }
      HRESULT status = E_FAIL;
      life->cleanupQueued = true;
      HRESULT call = active->RequestEditSession(
          client, cleanup, TF_ES_ASYNC | TF_ES_READWRITE, &status);
      cleanup->Release();
      if (FAILED(call) || FAILED(status)) {
        life->cleanupQueued = false;
        cancelPending = true;
        clearCore();
        return false;
      }
      if (life->live) {
        cancelPending = true;
        clearCore();
        return false;
      }
    }
    release(composition);
    cancelPending = false;
    clearCore();
    return true;
  }
  std::unique_ptr<tgskk::windows::Input> session;
  uint64_t inputEpoch = 0;
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
        found->second->live = false;
        lifetimes.erase(found);
      }
      if (composition == current)
        release(composition);
    }
    current->Release();
    return result;
  }
  bool eligible(uint32_t symbol, uint32_t modifiers) const {
    return composition || (session && session->mayProcess(symbol, modifiers));
  }
  static bool modifier(WPARAM key) {
    return key == VK_SHIFT || key == VK_CONTROL || key == VK_MENU ||
           key == VK_LWIN || key == VK_RWIN || key == VK_CAPITAL ||
           key == VK_NUMLOCK || key == VK_SCROLL ||
           (key >= VK_LSHIFT && key <= VK_RMENU);
  }
  bool editing = false;
  void clearCore() {
    if (session)
      session->reset();
    hideCandidates();
  }
  void hideCandidates() {
    if (candidates)
      ShowWindow(candidates, SW_HIDE);
  }
  const TgskkView *keyView(uint32_t symbol, uint32_t modifiers) {
    const auto *view = session->view();
    if (!view || !view->selecting || !view->candidate_count || modifiers)
      return session->key(symbol, modifiers);
    const size_t count = view->candidate_count;
    const size_t selected = view->selected;
    const bool list = selected >= inlineCandidates;
    if (symbol == TGSKK_KEY_PAGE_UP || symbol == TGSKK_KEY_PAGE_DOWN ||
        (list && (symbol == ' ' || symbol == 'x'))) {
      const auto move = tgskk::candidatePageMove(
          count, selected, inlineCandidates,
          symbol == ' ' || symbol == TGSKK_KEY_PAGE_DOWN, symbol == ' ');
      const auto *next = session->choose(move.index);
      if (next && move.kind == tgskk::CandidateMoveKind::Register)
        return session->key(' ', 0);
      if (next && move.kind == tgskk::CandidateMoveKind::ReturnToReading)
        return session->key('x', 0);
      return next;
    }
    if (symbol == TGSKK_KEY_UP || symbol == TGSKK_KEY_DOWN)
      return session->choose(
          tgskk::candidateStep(count, selected, symbol == TGSKK_KEY_DOWN));
    if (list && symbol >= '1' && symbol < '1' + pageSize) {
      const size_t index =
          tgskk::candidatePageStart(selected, inlineCandidates) + symbol - '1';
      return index < count ? session->select(index) : session->choose(selected);
    }
    return session->key(symbol, modifiers);
  }
  static constexpr size_t inlineCandidates = tgskk::defaultInlineCandidates;
  static constexpr size_t pageSize = tgskk::candidatePageSize;
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
    modifiers = ((state[VK_SHIFT] & 0x80) ? TGSKK_MOD_SHIFT : 0) |
                ((state[VK_CONTROL] & 0x80) ? TGSKK_MOD_CONTROL : 0) |
                ((state[VK_MENU] & 0x80) ? TGSKK_MOD_ALT : 0);
    if ((modifiers & TGSKK_MOD_ALT) || (state[VK_LWIN] & 0x80) ||
        (state[VK_RWIN] & 0x80))
      return false;
    switch (key) {
    case VK_RETURN:
      symbol = TGSKK_KEY_RETURN;
      break;
    case VK_BACK:
      symbol = TGSKK_KEY_BACKSPACE;
      break;
    case VK_ESCAPE:
      symbol = TGSKK_KEY_ESCAPE;
      break;
    case VK_TAB:
      symbol = TGSKK_KEY_TAB;
      break;
    case VK_LEFT:
      symbol = TGSKK_KEY_LEFT;
      break;
    case VK_UP:
      symbol = TGSKK_KEY_UP;
      break;
    case VK_RIGHT:
      symbol = TGSKK_KEY_RIGHT;
      break;
    case VK_DOWN:
      symbol = TGSKK_KEY_DOWN;
      break;
    case VK_PRIOR:
      symbol = TGSKK_KEY_PAGE_UP;
      break;
    case VK_NEXT:
      symbol = TGSKK_KEY_PAGE_DOWN;
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
      if ((modifiers & TGSKK_MOD_CONTROL) && !(modifiers & TGSKK_MOD_SHIFT) &&
          symbol >= 'A' && symbol <= 'Z')
        symbol += 'a' - 'A';
    }
    }
    return true;
  }
  HRESULT request(ITfContext *context, uint32_t symbol, uint32_t modifiers,
                  BOOL *eaten, bool reset = false) {
    auto *edit =
        new (std::nothrow) Edit(this, context, symbol, modifiers, reset);
    if (!edit)
      return E_OUTOFMEMORY;
    HRESULT result = E_FAIL;
    HRESULT call = context->RequestEditSession(
        client, edit, TF_ES_SYNC | TF_ES_READWRITE, &result);
    if (eaten)
      *eaten = SUCCEEDED(call) && edit->eaten;
    edit->Release();
    const HRESULT status = FAILED(call) ? call : result;
    return FAILED(status) ? reportFailure("edit request", status) : status;
  }

public:
  Service() { traceCount("service created", ++objects); }
  ~Service() {
    Deactivate();
    traceCount("service destroyed", --objects);
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
    if (!manager || n <= 2)
      traceCount("service released", n, "references");
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
    if (!value || id == TF_CLIENTID_NULL)
      return E_INVALIDARG;
    if (manager)
      return E_UNEXPECTED;
    if (flags & (TF_TMAE_SECUREMODE | TF_TMAE_UIELEMENTENABLEDONLY))
      return E_ACCESSDENIED;
    try {
      const auto broker = (std::filesystem::path(modulePath()).parent_path() /
                           L"tgskk-broker.exe")
                              .wstring();
      const auto config =
          tgskk::windows::Config::load(tgskk::windows::homeDirectory(), broker);
      session =
          std::make_unique<tgskk::windows::Input>(config, module, &objects);
      ++inputEpoch;
      manager = value;
      manager->AddRef();
      client = id;
      ITfKeystrokeMgr *keys = nullptr;
      HRESULT result = manager->QueryInterface(
          IID_ITfKeystrokeMgr, reinterpret_cast<void **>(&keys));
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
      if (!candidates) {
        const HRESULT error = HRESULT_FROM_WIN32(GetLastError());
        Deactivate();
        return FAILED(error) ? error : E_FAIL;
      }
      return S_OK;
    } catch (const std::exception &error) {
      reportCoreFailure("activate input", error.what());
      Deactivate();
      return E_FAIL;
    } catch (...) {
      Deactivate();
      return E_OUTOFMEMORY;
    }
  }
  HRESULT STDMETHODCALLTYPE Deactivate() override {
    traceCount("deactivate entered", refs.load(), "references");
    bool ended = abandon();
    unwatch();
    traceCount("after context unwatch", refs.load(), "references");
    if (ended)
      release(active);
    if (manager) {
      ITfKeystrokeMgr *keys = nullptr;
      if (SUCCEEDED(manager->QueryInterface(
              IID_ITfKeystrokeMgr, reinterpret_cast<void **>(&keys)))) {
        const HRESULT result = keys->UnadviseKeyEventSink(client);
        if (FAILED(result))
          reportFailure("unadvise key sink", result);
        release(keys);
        traceCount("after key sink removal", refs.load(), "references");
      }
      ITfSource *source = nullptr;
      if (threadCookie != TF_INVALID_COOKIE &&
          SUCCEEDED(manager->QueryInterface(
              IID_ITfSource, reinterpret_cast<void **>(&source)))) {
        HRESULT result = source->UnadviseSink(threadCookie);
        if (FAILED(result))
          reportFailure("unadvise thread sink", result);
        traceCount("after thread sink removal", refs.load(), "references");
        if (focusCookie != TF_INVALID_COOKIE) {
          result = source->UnadviseSink(focusCookie);
          if (FAILED(result))
            reportFailure("unadvise focus sink", result);
        }
        traceCount("after focus sink removal", refs.load(), "references");
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
    session.reset();
    ++inputEpoch;
    traceCount("deactivate completed", refs.load(), "references");
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
    if (composition && session && !disabled(context)) {
      *eaten = TRUE;
      return S_OK;
    }
    if (session && !disabled(context) &&
        translate(key, scan, symbol, modifiers))
      *eaten = eligible(symbol, modifiers);
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
    if (!session || disabled(context) ||
        !translate(key, scan, symbol, modifiers)) {
      BOOL ignored = FALSE;
      if (!composition || context != active ||
          FAILED(request(context, 0, 0, &ignored, true)))
        abandon(true);
      return S_OK;
    }
    if (!eligible(symbol, modifiers))
      return S_OK;
    if (cancelPending && context != active && !abandon())
      return S_OK;
    if (active != context) {
      if (!abandon())
        return S_OK;
      unwatch();
      release(active);
      active = context;
      active->AddRef();
      ITfSource *source = nullptr;
      if (SUCCEEDED(active->QueryInterface(
              IID_ITfSource, reinterpret_cast<void **>(&source)))) {
        HRESULT watched = source->AdviseSink(
            IID_ITfTextEditSink, static_cast<ITfTextEditSink *>(this),
            &editCookie);
        release(source);
        if (FAILED(watched)) {
          editCookie = TF_INVALID_COOKIE;
          release(active);
          return S_OK;
        }
      }
      if (editCookie == TF_INVALID_COOKIE) {
        release(active);
        return S_OK;
      }
    }
    const auto requestEpoch = inputEpoch;
    HRESULT result = request(context, symbol, modifiers, eaten);
    if (FAILED(result) && inputEpoch == requestEpoch && active == context)
      abandon(true);
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
  OnCompositionTerminated(TfEditCookie cookie, ITfComposition *value) override {
    auto found = lifetimes.find(value);
    std::shared_ptr<CompositionLife> life;
    if (found != lifetimes.end()) {
      life = found->second;
      life->live = false;
      lifetimes.erase(found);
    }
    HRESULT result = S_OK;
    if (!editing && life && life->erase) {
      editing = true;
      result = eraseComposition(value, cookie, life->preedit);
      editing = false;
    }
    if (value == composition) {
      release(composition);
      cancelPending = false;
      if (!editing)
        clearCore();
      if (!manager)
        release(active);
    }
    if (FAILED(result))
      reportFailure("forced composition termination", result);
    // The notification is complete even when restoring the caret failed.
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnSetThreadFocus() override {
    if (cancelPending)
      abandon();
    return S_OK;
  }
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
    if (active == context && abandon()) {
      unwatch();
      release(active);
    }
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnSetFocus(ITfDocumentMgr *,
                                       ITfDocumentMgr *) override {
    if (abandon()) {
      unwatch();
      release(active);
    }
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
  void recover(TfEditCookie cookie) noexcept {
    editing = false;
    if (composition)
      finish(cookie);
    clearCore();
  }
  HRESULT edit(ITfContext *context, TfEditCookie cookie, uint32_t symbol,
               uint32_t modifiers, bool reset, bool &eaten) {
    eaten = false;
    if (!session || (active && context != active))
      return TF_E_DISCONNECTED;
    const auto epoch = inputEpoch;
    ITfComposition *expectedComposition = composition;
    std::shared_ptr<CompositionLife> expectedLife;
    const auto sameActivation = [&] {
      return session && inputEpoch == epoch && active == context;
    };
    const auto currentInput = [&] {
      return sameActivation() && composition == expectedComposition &&
             (!expectedLife || expectedLife->live);
    };
    if (cancelPending && composition) {
      auto found = lifetimes.find(composition);
      if (found == lifetimes.end())
        return E_UNEXPECTED;
      const auto life = found->second;
      HRESULT result;
      {
        Cleanup cleanup(composition, life, life->erase);
        result = cleanup.DoEditSession(cookie);
      }
      if (FAILED(result))
        return result;
      release(composition);
      cancelPending = false;
      clearCore();
    }
    expectedComposition = composition;
    if (auto found = lifetimes.find(composition); found != lifetimes.end())
      expectedLife = found->second;
    if (!currentInput())
      return TF_E_DISCONNECTED;
    ITfRange *range = nullptr;
    bool fresh = !composition;
    if (composition) {
      const HRESULT result = composition->GetRange(&range);
      if (FAILED(result))
        return reportFailure("composition range", result);
    }
    if (!range) {
      TF_SELECTION selection{};
      ULONG fetched = 0;
      HRESULT hr = context->GetSelection(cookie, TF_DEFAULT_SELECTION, 1,
                                         &selection, &fetched);
      if (FAILED(hr) || fetched != 1 || !selection.range)
        return reportFailure("selection", FAILED(hr) ? hr : E_FAIL);
      range = selection.range;
    }
    if (!currentInput()) {
      release(range);
      return TF_E_DISCONNECTED;
    }
    session->context("", false, false);
    bool knownScope = false;
    if (!reset) {
      ITfReadOnlyProperty *property = nullptr;
      HRESULT scopeResult =
          context->GetAppProperty(GUID_PROP_INPUTSCOPE, &property);
      if (FAILED(scopeResult) || !property) {
        release(range);
        return reportFailure("input scope property",
                             FAILED(scopeResult) ? scopeResult : E_FAIL);
      }
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
            knownScope = SUCCEEDED(scopeResult) && count > 0;
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
          return reportFailure("input scope", FAILED(scopeResult)
                                                  ? scopeResult
                                                  : E_ACCESSDENIED);
        }
      }
    }
    if (!reset && knownScope) {
      TF_SELECTION current{};
      ULONG fetched = 0;
      BOOL collapsed = FALSE;
      const bool permitted =
          SUCCEEDED(context->GetSelection(cookie, TF_DEFAULT_SELECTION, 1,
                                          &current, &fetched)) &&
          fetched == 1 && current.range &&
          SUCCEEDED(current.range->IsEmpty(cookie, &collapsed)) && collapsed;
      release(current.range);
      if (permitted) {
        ITfRange *left = nullptr;
        std::wstring surrounding;
        if (SUCCEEDED(range->Clone(&left))) {
          LONG moved = 0;
          if (SUCCEEDED(left->Collapse(cookie, TF_ANCHOR_START)) &&
              SUCCEEDED(left->ShiftStart(cookie, -256, &moved, nullptr))) {
            wchar_t text[256];
            ULONG count = 0;
            if (SUCCEEDED(left->GetText(cookie, 0, text, 256, &count)))
              surrounding.assign(text, count);
          }
          release(left);
        }
        if (!surrounding.empty() && surrounding.front() >= 0xdc00 &&
            surrounding.front() <= 0xdfff)
          surrounding.erase(0, 1);
        if (!surrounding.empty() && surrounding.back() >= 0xd800 &&
            surrounding.back() <= 0xdbff)
          surrounding.pop_back();
        if (!currentInput()) {
          release(range);
          return TF_E_DISCONNECTED;
        }
        session->context(utf8(surrounding), true, true);
      }
    }
    if (!currentInput()) {
      release(range);
      return TF_E_DISCONNECTED;
    }
    if (reset && fresh) {
      clearCore();
      release(range);
      return S_OK;
    }
    std::erase_if(lifetimes,
                  [](const auto &entry) { return !entry.second->live; });
    editing = true;
    if (fresh) {
      ITfContextComposition *owner = nullptr;
      ITfComposition *started = nullptr;
      HRESULT hr = context->QueryInterface(IID_ITfContextComposition,
                                           reinterpret_cast<void **>(&owner));
      if (SUCCEEDED(hr)) {
        hr = owner->StartComposition(cookie, range, this, &started);
        release(owner);
      }
      if (!currentInput()) {
        if (started)
          started->EndComposition(cookie);
        release(started);
        editing = false;
        release(range);
        return TF_E_DISCONNECTED;
      }
      if (FAILED(hr) || !started) {
        release(started);
        editing = false;
        release(range);
        return reportFailure("start composition", FAILED(hr) ? hr : E_FAIL);
      }
      composition = started;
      expectedComposition = composition;
      expectedLife = std::make_shared<CompositionLife>();
      lifetimes[composition] = expectedLife;
    }
    const TgskkView *view =
        reset ? session->reset() : keyView(symbol, modifiers);
    if (!view) {
      if (session->takeTransportFailure()) {
        const std::string error = session->error();
        ITfComposition *failed = composition;
        if (failed)
          failed->AddRef();
        if (auto found = lifetimes.find(failed);
            failed && found != lifetimes.end()) {
          const auto preedit = found->second->preedit;
          eraseComposition(failed, cookie, preedit);
        }
        if (currentInput())
          finish(cookie);
        release(failed);
        reportCoreFailure("broker request", error.c_str());
      }
      editing = false;
      release(range);
      return E_FAIL;
    }
    if (const auto diagnostic = session->takeDiagnostic(); !diagnostic.empty())
      reportCoreFailure("broker status", diagnostic.c_str());
    bool consumed = view->consumed;
    if (!consumed && !*view->committed && !reset) {
      reset = true;
      view = session->reset();
      if (!view) {
        editing = false;
        release(range);
        return E_FAIL;
      }
    }
    // Application callbacks can deactivate the service and invalidate its view.
    const TgskkView snapshot = *view;
    view = &snapshot;
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
      if (SUCCEEDED(result)) {
        eaten = consumed;
        auto found = lifetimes.find(composition);
        if (found != lifetimes.end())
          found->second->preedit = preedit;
      }
      if (SUCCEEDED(result) && !preedit.empty() && !composition)
        result = TF_E_DISCONNECTED;
      if (SUCCEEDED(result) && preedit.empty()) {
        result = finish(cookie);
        if (SUCCEEDED(result) && sameActivation() && !composition) {
          expectedComposition = nullptr;
          expectedLife.reset();
        }
      } else if (SUCCEEDED(result) && !committed.empty()) {
        ITfComposition *current = composition;
        current->AddRef();
        ITfRange *tail = nullptr;
        result = range->Clone(&tail);
        if (SUCCEEDED(result)) {
          LONG shifted = 0;
          result = tail->ShiftStart(cookie, static_cast<LONG>(committed.size()),
                                    &shifted, nullptr);
          if (SUCCEEDED(result)) {
            if (composition != current ||
                shifted != static_cast<LONG>(committed.size()))
              result = TF_E_DISCONNECTED;
            else
              result = current->ShiftStart(cookie, tail);
          }
          release(tail);
        }
        current->Release();
      }
      if (SUCCEEDED(result) && changed) {
        range->Collapse(cookie, TF_ANCHOR_END);
        TF_SELECTION selection{range, {TF_AE_NONE, FALSE}};
        result = context->SetSelection(cookie, 1, &selection);
      }
      hideCandidates();
      if (!currentInput())
        result = TF_E_DISCONNECTED;
      if (SUCCEEDED(result) && candidates && view->selecting &&
          view->selected >= inlineCandidates) {
        std::wstring text = view->zenz_applied ? L"AI-ranked\r\n" : L"";
        const size_t start =
            tgskk::candidatePageStart(view->selected, inlineCandidates);
        for (size_t i = start;
             i < view->candidate_count && i < start + pageSize; ++i) {
          text += (i == view->selected ? L"> " : L"  ") +
                  std::to_wstring(i - start + 1) + L" " +
                  wide(session->candidate(i));
          const char *annotation = session->candidate(i, true);
          if (annotation && *annotation)
            text += L" " + wide(annotation);
          text += L"\r\n";
        }
        ITfContextView *viewport = nullptr;
        if (SUCCEEDED(context->GetActiveView(&viewport))) {
          RECT caret{};
          BOOL clipped = FALSE;
          if (SUCCEEDED(
                  viewport->GetTextExt(cookie, range, &caret, &clipped))) {
            SetWindowTextW(candidates, text.c_str());
            SetWindowPos(
                candidates, HWND_TOPMOST, caret.left, caret.bottom, 320,
                static_cast<int>(
                    (std::min(view->candidate_count - start, pageSize) +
                     (view->zenz_applied ? 1 : 0)) *
                        24 +
                    8),
                SWP_NOACTIVATE | SWP_SHOWWINDOW);
          }
          release(viewport);
        }
      }
    }
    if (!reset && !preedit.empty() && !composition) {
      clearCore();
      result = TF_E_DISCONNECTED;
    }
    editing = false;
    release(range);
    return FAILED(result) ? reportFailure("apply edit", result) : result;
  }
};
Edit::Edit(Service *value, ITfContext *ctx, uint32_t key, uint32_t mods,
           bool clear)
    : owner(value), context(ctx), symbol(key), modifiers(mods), reset(clear) {
  owner->AddRef();
  context->AddRef();
  traceCount("edit created", ++objects);
}
Edit::~Edit() {
  context->Release();
  owner->Release();
  traceCount("edit destroyed", --objects);
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
  try {
    return owner->edit(context, cookie, symbol, modifiers, reset, eaten);
  } catch (...) {
    owner->recover(cookie);
    return E_OUTOFMEMORY;
  }
}

class Factory final : public IClassFactory {
  std::atomic<ULONG> refs{1};

public:
  Factory() { traceCount("factory created", ++objects); }
  ~Factory() { traceCount("factory destroyed", --objects); }
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
    if (!out)
      return E_POINTER;
    *out = nullptr;
    auto *service = new (std::nothrow) Service;
    if (!service)
      return E_OUTOFMEMORY;
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
std::wstring classKey() {
  wchar_t guid[64];
  StringFromGUID2(serviceId, guid, 64);
  return std::wstring(L"Software\\Classes\\CLSID\\") + guid;
}
HRESULT existingRegistration() {
  auto key = classKey() + L"\\InprocServer32";
  wchar_t path[32768];
  DWORD bytes = sizeof(path);
  LSTATUS status = RegGetValueW(HKEY_LOCAL_MACHINE, key.c_str(), nullptr,
                                RRF_RT_REG_SZ, nullptr, path, &bytes);
  if (status == ERROR_FILE_NOT_FOUND || status == ERROR_PATH_NOT_FOUND)
    return S_FALSE;
  if (status != ERROR_SUCCESS)
    return HRESULT_FROM_WIN32(status);
  return std::wstring(path) == modulePath()
             ? S_OK
             : HRESULT_FROM_WIN32(ERROR_ALREADY_EXISTS);
}
HRESULT registry(bool install) {
  std::wstring key = classKey();
  if (!install) {
    auto status = RegDeleteTreeW(HKEY_LOCAL_MACHINE, key.c_str());
    return status == ERROR_FILE_NOT_FOUND || status == ERROR_SUCCESS
               ? S_OK
               : HRESULT_FROM_WIN32(status);
  }
  auto path = modulePath();
  if (path.empty())
    return E_FAIL;
  key += L"\\InprocServer32";
  HKEY handle;
  auto status = RegCreateKeyExW(HKEY_LOCAL_MACHINE, key.c_str(), 0, nullptr, 0,
                                KEY_WRITE, nullptr, &handle, nullptr);
  if (status != ERROR_SUCCESS)
    return HRESULT_FROM_WIN32(status);
  status = RegSetValueExW(
      handle, nullptr, 0, REG_SZ, reinterpret_cast<const BYTE *>(path.c_str()),
      static_cast<DWORD>((path.size() + 1) * sizeof(wchar_t)));
  if (status == ERROR_SUCCESS)
    status = RegSetValueExW(handle, L"ThreadingModel", 0, REG_SZ,
                            reinterpret_cast<const BYTE *>(L"Apartment"),
                            sizeof(L"Apartment"));
  RegCloseKey(handle);
  if (status != ERROR_SUCCESS)
    RegDeleteTreeW(HKEY_LOCAL_MACHINE, classKey().c_str());
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
  if (objects != 0)
    traceCount("unload pending", objects.load());
  return objects == 0 && locks == 0 ? S_OK : S_FALSE;
}
extern "C" HRESULT __stdcall DllGetClassObject(REFCLSID id, REFIID iid,
                                               void **out) {
  if (!out)
    return E_POINTER;
  *out = nullptr;
  if (id != serviceId)
    return CLASS_E_CLASSNOTAVAILABLE;
  auto *factory = new (std::nothrow) Factory;
  if (!factory)
    return E_OUTOFMEMORY;
  auto result = factory->QueryInterface(iid, out);
  factory->Release();
  return result;
}
extern "C" HRESULT __stdcall DllRegisterServer() {
  HRESULT initialized = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
  if (FAILED(initialized) && initialized != RPC_E_CHANGED_MODE)
    return initialized;
  ITfInputProcessorProfiles *profiles = nullptr;
  ITfCategoryMgr *categories = nullptr;
  bool createdRegistry = false, registeredProfile = false;
  HRESULT result = E_FAIL;
  try {
    result = existingRegistration();
    if (SUCCEEDED(result)) {
      const bool fresh = result == S_FALSE;
      result = fresh ? registry(true) : S_OK;
      createdRegistry = fresh && SUCCEEDED(result);
      if (SUCCEEDED(result))
        result = CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr,
                                  CLSCTX_INPROC_SERVER,
                                  IID_ITfInputProcessorProfiles,
                                  reinterpret_cast<void **>(&profiles));
      if (SUCCEEDED(result)) {
        result = profiles->Register(serviceId);
        registeredProfile = SUCCEEDED(result);
      }
      if (SUCCEEDED(result)) {
        auto path = modulePath();
        result = profiles->AddLanguageProfile(
            serviceId, MAKELANGID(LANG_JAPANESE, SUBLANG_DEFAULT), profileId,
            L"tgskk", 5, path.c_str(), static_cast<ULONG>(path.size()), 0);
      }
      if (SUCCEEDED(result))
        result = CoCreateInstance(CLSID_TF_CategoryMgr, nullptr,
                                  CLSCTX_INPROC_SERVER, IID_ITfCategoryMgr,
                                  reinterpret_cast<void **>(&categories));
      if (SUCCEEDED(result))
        result = categories->RegisterCategory(
            serviceId, GUID_TFCAT_TIP_KEYBOARD, serviceId);
    }
  } catch (...) {
    result = E_OUTOFMEMORY;
  }
  if (FAILED(result) && createdRegistry) {
    HRESULT cleanup = S_OK;
    if (categories)
      cleanup = categories->UnregisterCategory(
          serviceId, GUID_TFCAT_TIP_KEYBOARD, serviceId);
    if (registeredProfile) {
      const HRESULT removed = profiles->Unregister(serviceId);
      if (FAILED(removed))
        cleanup = removed;
    }
    // Retain the class path when cleanup fails so regsvr32 /u can retry.
    if (SUCCEEDED(cleanup)) {
      try {
        cleanup = registry(false);
      } catch (...) {
        cleanup = E_OUTOFMEMORY;
      }
    }
    if (FAILED(cleanup))
      OutputDebugStringW(
          L"tgskk: partial registration cleanup failed; run regsvr32 /u.\n");
  }
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
  ITfCategoryMgr *categories = nullptr;
  HRESULT result = E_FAIL;
  try {
    result = existingRegistration();
    if (result == S_FALSE) {
      result = S_OK;
    } else if (SUCCEEDED(result)) {
      result = CoCreateInstance(CLSID_TF_CategoryMgr, nullptr,
                                CLSCTX_INPROC_SERVER, IID_ITfCategoryMgr,
                                reinterpret_cast<void **>(&categories));
      if (SUCCEEDED(result))
        result = categories->UnregisterCategory(
            serviceId, GUID_TFCAT_TIP_KEYBOARD, serviceId);
      HRESULT profileResult = CoCreateInstance(
          CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
          IID_ITfInputProcessorProfiles, reinterpret_cast<void **>(&profiles));
      if (SUCCEEDED(profileResult))
        profileResult = profiles->Unregister(serviceId);
      if (SUCCEEDED(result))
        result = profileResult;
      if (SUCCEEDED(result))
        result = registry(false);
    }
  } catch (...) {
    result = E_OUTOFMEMORY;
  }
  release(categories);
  release(profiles);
  if (SUCCEEDED(initialized))
    CoUninitialize();
  return result;
}
