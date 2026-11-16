// SPDX-License-Identifier: GPL-3.0-or-later
#include "../tsf_compat.h"
#include "../ime_controls.h"
#include <algorithm>
#include <atomic>
#include <cstdio>
#include <cstdlib>
#include <exception>
#include <functional>
#include <inputscope.h>
#include <iterator>
#include <memory>
#include <msctf.h>
#include <stdexcept>
#include <string>
#include <textstor.h>
#include <thread>
#include <utility>
#include <windows.h>

static void require(bool condition, const char *reason) {
  if (!condition) {
    throw std::runtime_error(reason);
  }
}
const CLSID serviceId = {0xc7993bd4,
                         0x1094,
                         0x4968,
                         {0xa3, 0x11, 0x3d, 0xb8, 0xa0, 0xe0, 0x17, 0x39}};
const GUID profileId = {0x70c846ab,
                        0x9fb5,
                        0x43cb,
                        {0x94, 0xbb, 0x74, 0x83, 0xb9, 0xb2, 0xf6, 0xf8}};
class Scope final : public ITfInputScope {
  ULONG refs = 1;
  InputScope value;
  std::shared_ptr<std::function<void()>> onRead;

public:
  explicit Scope(InputScope scope,
                 std::shared_ptr<std::function<void()>> callback = {})
      : value(scope), onRead(std::move(callback)) {}
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void **out) override {
    if (!out)
      return E_POINTER;
    *out = nullptr;
    if (id != IID_IUnknown && id != IID_ITfInputScope)
      return E_NOINTERFACE;
    *out = static_cast<ITfInputScope *>(this);
    AddRef();
    return S_OK;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++refs; }
  ULONG STDMETHODCALLTYPE Release() override {
    ULONG remaining = --refs;
    if (!remaining)
      delete this;
    return remaining;
  }
  HRESULT STDMETHODCALLTYPE GetInputScopes(InputScope **out,
                                           UINT *count) override {
    if (onRead)
      if (auto callback = std::exchange(*onRead, {}))
        callback();
    *count = 0;
    *out = static_cast<InputScope *>(CoTaskMemAlloc(sizeof(InputScope)));
    if (!*out)
      return E_OUTOFMEMORY;
    **out = value;
    *count = 1;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetPhrase(BSTR **out, UINT *count) override {
    *out = nullptr;
    *count = 0;
    return E_NOTIMPL;
  }
  HRESULT STDMETHODCALLTYPE GetRegularExpression(BSTR *out) override {
    *out = nullptr;
    return E_NOTIMPL;
  }
  HRESULT STDMETHODCALLTYPE GetSRGS(BSTR *out) override {
    *out = nullptr;
    return E_NOTIMPL;
  }
  HRESULT STDMETHODCALLTYPE GetXML(BSTR *out) override {
    *out = nullptr;
    return E_NOTIMPL;
  }
};
class Store final : public ITextStoreACP,
                    public ITfContextOwnerCompositionSink {
  ULONG refs = 1;
  ITextStoreACPSink *sink = nullptr;
  bool locked = false;
  DWORD pendingLock = 0;
  bool scopeRequested = false;
  void attributes(ULONG count, const TS_ATTRID *ids) {
    scopeRequested = false;
    for (ULONG i = 0; i < count; ++i)
      scopeRequested |= ids[i] == GUID_PROP_INPUTSCOPE;
  }

public:
  std::wstring text;
  LONG start = 0, end = 0;
  bool refuseComposition = false, failSelection = false, denyLock = false;
  bool failSelectionAfterWrite = false;
  bool holdAsync = false;
  InputScope scope = IS_DEFAULT;
  bool scopeFailure = false;
  std::shared_ptr<std::function<void()>> scopeRead =
      std::make_shared<std::function<void()>>();
  std::shared_ptr<std::function<void()>> textExtRead =
      std::make_shared<std::function<void()>>();
  RECT textExt{0, 0, 10, 20};
  ULONG endedCompositions = 0, startedCompositions = 0;
  HWND window = nullptr;
  ~Store() {
    std::fputs("test store destroyed\n", stderr);
    if (sink)
      sink->Release();
  }
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void **out) override {
    if (!out)
      return E_POINTER;
    *out = nullptr;
    if (id == IID_IUnknown || id == IID_ITextStoreACP)
      *out = static_cast<ITextStoreACP *>(this);
    else if (id == IID_ITfContextOwnerCompositionSink)
      *out = static_cast<ITfContextOwnerCompositionSink *>(this);
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
  HRESULT STDMETHODCALLTYPE AdviseSink(REFIID id, IUnknown *value,
                                       DWORD) override {
    if (sink)
      return E_UNEXPECTED;
    return value->QueryInterface(id, reinterpret_cast<void **>(&sink));
  }
  HRESULT STDMETHODCALLTYPE UnadviseSink(IUnknown *) override {
    std::fputs("test store unadvised\n", stderr);
    if (sink) {
      sink->Release();
      sink = nullptr;
    }
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE RequestLock(DWORD flags, HRESULT *result) override {
    if (!sink)
      return E_FAIL;
    if (denyLock) {
      *result = TS_E_SYNCHRONOUS;
      return S_OK;
    }
    if (holdAsync) {
      // An asynchronous TSF request may first try to obtain a synchronous lock.
      if (flags & TS_LF_SYNC)
        *result = TS_E_SYNCHRONOUS;
      else {
        pendingLock |= flags;
        *result = TS_S_ASYNC;
      }
      return S_OK;
    }
    if (locked) {
      if (flags & TS_LF_SYNC)
        *result = TS_E_SYNCHRONOUS;
      else {
        pendingLock |= flags;
        *result = TS_S_ASYNC;
      }
      return S_OK;
    }
    locked = true;
    *result = sink->OnLockGranted(flags);
    locked = false;
    while (pendingLock && !holdAsync) {
      DWORD next = pendingLock;
      pendingLock = 0;
      locked = true;
      sink->OnLockGranted(next);
      locked = false;
    }
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetStatus(TS_STATUS *value) override {
    *value = {0, TS_SS_NOHIDDENTEXT};
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE QueryInsert(LONG a, LONG b, ULONG, LONG *x,
                                        LONG *y) override {
    *x = a;
    *y = b;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetSelection(ULONG, ULONG count,
                                         TS_SELECTION_ACP *selection,
                                         ULONG *fetched) override {
    *fetched = count ? 1 : 0;
    if (count)
      *selection = {start, end, {TS_AE_END, FALSE}};
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE
  SetSelection(ULONG count, const TS_SELECTION_ACP *selection) override {
    if (failSelection)
      return E_FAIL;
    if (count != 1)
      return E_INVALIDARG;
    start = selection->acpStart;
    end = selection->acpEnd;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetText(LONG a, LONG b, WCHAR *out, ULONG capacity,
                                    ULONG *length, TS_RUNINFO *runs,
                                    ULONG runCapacity, ULONG *runLength,
                                    LONG *next) override {
    if (b == -1)
      b = static_cast<LONG>(text.size());
    if (a < 0 || b < a || b > static_cast<LONG>(text.size()))
      return TS_E_INVALIDPOS;
    ULONG count = std::min<ULONG>(b - a, capacity);
    ULONG runCount = capacity ? count : static_cast<ULONG>(b - a);
    if (out && count)
      std::copy_n(text.data() + a, count, out);
    if (length)
      *length = count;
    if (runLength)
      *runLength = runCapacity && runCount ? 1 : 0;
    if (runCapacity && runCount)
      runs[0] = {runCount, TS_RT_PLAIN};
    if (next)
      *next = a + (runCapacity ? runCount : count);
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE SetText(DWORD, LONG a, LONG b, const WCHAR *value,
                                    ULONG count,
                                    TS_TEXTCHANGE *change) override {
    if (a < 0 || b < a || b > static_cast<LONG>(text.size()))
      return TS_E_INVALIDPOS;
    text.replace(a, b - a, value, count);
    start = end = a + static_cast<LONG>(count);
    if (failSelectionAfterWrite) {
      failSelectionAfterWrite = false;
      failSelection = true;
    }
    *change = {a, b, a + static_cast<LONG>(count)};
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetFormattedText(LONG, LONG,
                                             IDataObject **) override {
    return E_NOTIMPL;
  }
  HRESULT STDMETHODCALLTYPE GetEmbedded(LONG, REFGUID, REFIID,
                                        IUnknown **) override {
    return E_NOTIMPL;
  }
  HRESULT STDMETHODCALLTYPE QueryInsertEmbedded(const GUID *, const FORMATETC *,
                                                BOOL *value) override {
    *value = FALSE;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE InsertEmbedded(DWORD, LONG, LONG, IDataObject *,
                                           TS_TEXTCHANGE *) override {
    return E_NOTIMPL;
  }
  HRESULT STDMETHODCALLTYPE
  InsertTextAtSelection(DWORD flags, const WCHAR *value, ULONG count, LONG *a,
                        LONG *b, TS_TEXTCHANGE *change) override {
    if (a)
      *a = start;
    if (b)
      *b = end;
    if (flags & TS_IAS_QUERYONLY)
      return S_OK;
    TS_TEXTCHANGE local;
    HRESULT result =
        SetText(0, start, end, value, count, change ? change : &local);
    if (SUCCEEDED(result) && b)
      *b = end;
    return result;
  }
  HRESULT STDMETHODCALLTYPE InsertEmbeddedAtSelection(
      DWORD, IDataObject *, LONG *, LONG *, TS_TEXTCHANGE *) override {
    return E_NOTIMPL;
  }
  HRESULT STDMETHODCALLTYPE
  RequestSupportedAttrs(DWORD, ULONG count, const TS_ATTRID *ids) override {
    attributes(count, ids);
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE RequestAttrsAtPosition(LONG, ULONG count,
                                                   const TS_ATTRID *ids,
                                                   DWORD) override {
    attributes(count, ids);
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE RequestAttrsTransitioningAtPosition(
      LONG, ULONG, const TS_ATTRID *, DWORD) override {
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE FindNextAttrTransition(LONG, LONG halt, ULONG,
                                                   const TS_ATTRID *, DWORD,
                                                   LONG *next, BOOL *found,
                                                   LONG *offset) override {
    *next = halt;
    *found = FALSE;
    *offset = 0;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE RetrieveRequestedAttrs(ULONG capacity,
                                                   TS_ATTRVAL *values,
                                                   ULONG *count) override {
    *count = 0;
    if (scopeFailure)
      return E_FAIL;
    if (scopeRequested && capacity) {
      values[0].idAttr = GUID_PROP_INPUTSCOPE;
      values[0].dwOverlapId = 0;
      VariantInit(&values[0].varValue);
      values[0].varValue.vt = VT_UNKNOWN;
      values[0].varValue.punkVal = new Scope(scope, scopeRead);
      *count = 1;
      scopeRequested = false;
    }
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetEndACP(LONG *value) override {
    *value = static_cast<LONG>(text.size());
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetActiveView(TsViewCookie *value) override {
    *value = 1;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetACPFromPoint(TsViewCookie, const POINT *, DWORD,
                                            LONG *value) override {
    *value = end;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetTextExt(TsViewCookie, LONG, LONG, RECT *rect,
                                       BOOL *clipped) override {
    if (auto callback = std::exchange(*textExtRead, {}))
      callback();
    *rect = textExt;
    *clipped = FALSE;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetScreenExt(TsViewCookie, RECT *rect) override {
    *rect = {0, 0, 800, 600};
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetWnd(TsViewCookie, HWND *value) override {
    *value = window;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnStartComposition(ITfCompositionView *,
                                               BOOL *allowed) override {
    ++startedCompositions;
    *allowed = !refuseComposition;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnUpdateComposition(ITfCompositionView *,
                                                ITfRange *) override {
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnEndComposition(ITfCompositionView *) override {
    ++endedCompositions;
    return S_OK;
  }
  void move(LONG position) {
    start = end = position;
    if (sink)
      sink->OnSelectionChange();
  }
  void replace(const std::wstring &value) {
    TS_TEXTCHANGE change{0, static_cast<LONG>(text.size()),
                         static_cast<LONG>(value.size())};
    text = value;
    start = end = static_cast<LONG>(text.size());
    if (sink) {
      sink->OnTextChange(0, &change);
      sink->OnSelectionChange();
    }
  }
  void setScope(InputScope value, bool fail = false) {
    scope = value;
    scopeFailure = fail;
    if (sink)
      sink->OnAttrsChange(0, static_cast<LONG>(text.size()), 1,
                          &GUID_PROP_INPUTSCOPE);
  }
  void appInsert(const std::wstring &value) {
    TS_TEXTCHANGE change{start, end, start + static_cast<LONG>(value.size())};
    text.replace(start, end - start, value);
    start = end = change.acpNewEnd;
    if (sink) {
      sink->OnTextChange(0, &change);
      sink->OnSelectionChange();
    }
  }
  void grantAsync() {
    holdAsync = false;
    while (pendingLock) {
      DWORD flags = pendingLock;
      pendingLock = 0;
      locked = true;
      sink->OnLockGranted(flags);
      locked = false;
    }
  }
};
void pump() {
  MSG msg;
  for (unsigned i = 0; i < 20; ++i) {
    while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) {
      TranslateMessage(&msg);
      DispatchMessageW(&msg);
    }
    Sleep(2);
  }
}
struct PopupSearch {
  const wchar_t *label;
  HWND window = nullptr;
};
BOOL CALLBACK findPopup(HWND window, LPARAM data) {
  auto *search = reinterpret_cast<PopupSearch *>(data);
  wchar_t className[16], text[32];
  const int classLength =
      GetClassNameW(window, className, static_cast<int>(std::size(className)));
  const int textLength =
      GetWindowTextW(window, text, static_cast<int>(std::size(text)));
  if (classLength > 0 && textLength > 0 &&
      CompareStringOrdinal(className, classLength, L"STATIC", -1, TRUE) ==
          CSTR_EQUAL &&
      std::wstring(text, static_cast<size_t>(textLength)) == search->label) {
    search->window = window;
    return FALSE;
  }
  return TRUE;
}
HWND findModePopup(const wchar_t *label) {
  PopupSearch search{label};
  EnumThreadWindows(GetCurrentThreadId(), findPopup,
                    reinterpret_cast<LPARAM>(&search));
  return search.window;
}
void idleEditingKeys(ITfKeyEventSink *keys, ITfContext *context, Store *store) {
  for (unsigned modifiers : {0u, 1u, 2u, 3u}) {
    BYTE state[256] = {};
    if (modifiers & 1)
      state[VK_SHIFT] = state[VK_LSHIFT] = 0x80;
    if (modifiers & 2)
      state[VK_CONTROL] = state[VK_LCONTROL] = 0x80;
    require(SetKeyboardState(state), "application editing key state");
    for (UINT code :
         {VK_BACK, VK_RETURN, VK_TAB, VK_ESCAPE, VK_LEFT, VK_UP, VK_RIGHT,
          VK_DOWN, VK_PRIOR, VK_NEXT, VK_HOME, VK_END, VK_DELETE}) {
      const LPARAM scan = 1 | (MapVirtualKeyW(code, MAPVK_VK_TO_VSC) << 16);
      const auto before = store->startedCompositions;
      BOOL eligible = TRUE, consumed = TRUE;
      require(SUCCEEDED(keys->OnTestKeyDown(context, code, scan, &eligible)) &&
                  !eligible,
              "idle application editing key was eligible");
      require(SUCCEEDED(keys->OnKeyDown(context, code, scan, &consumed)) &&
                  !consumed,
              "idle application editing key was consumed");
      require(store->startedCompositions == before,
              "idle application editing key started a composition");
    }
  }
  BYTE clear[256] = {};
  require(SetKeyboardState(clear), "restore editing key state");
}
struct RegistrationGuard {
  HRESULT(WINAPI *remove)();
  bool active = false;
  ~RegistrationGuard() {
    if (active && FAILED(remove()))
      std::fputs("TSF cleanup failed; run regsvr32 /u on the test DLL.\n",
                 stderr);
  }
};
class CapturingFactory final : public IClassFactory {
  std::atomic<ULONG> refs{1};
  IClassFactory *target;

public:
  ITfKeyEventSink *sink = nullptr;
  explicit CapturingFactory(IClassFactory *value) : target(value) {}
  ~CapturingFactory() {
    if (sink)
      sink->Release();
    target->Release();
  }
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void **out) override {
    if (!out)
      return E_POINTER;
    *out = nullptr;
    if (id != IID_IUnknown && id != IID_IClassFactory)
      return E_NOINTERFACE;
    *out = static_cast<IClassFactory *>(this);
    AddRef();
    return S_OK;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++refs; }
  ULONG STDMETHODCALLTYPE Release() override {
    const auto count = --refs;
    if (!count)
      delete this;
    return count;
  }
  HRESULT STDMETHODCALLTYPE CreateInstance(IUnknown *outer, REFIID id,
                                           void **out) override {
    const auto result = target->CreateInstance(outer, id, out);
    if (SUCCEEDED(result)) {
      if (sink)
        sink->Release();
      sink = nullptr;
      static_cast<IUnknown *>(*out)->QueryInterface(
          IID_ITfKeyEventSink, reinterpret_cast<void **>(&sink));
    }
    return result;
  }
  HRESULT STDMETHODCALLTYPE LockServer(BOOL lock) override {
    return target->LockServer(lock);
  }
};
struct CaptureRegistration {
  struct ReleaseFactory {
    void operator()(CapturingFactory *value) const { value->Release(); }
  };
  std::unique_ptr<CapturingFactory, ReleaseFactory> factory;
  DWORD cookie = 0;
  HRESULT close() {
    if (cookie) {
      const auto result = CoRevokeClassObject(cookie);
      if (FAILED(result))
        return result;
      cookie = 0;
    }
    factory.reset();
    return S_OK;
  }
  ~CaptureRegistration() { close(); }
};
std::atomic<bool> refusalProbeAlive = false;
class RefusalProbe final : public ITfEditSession, public ITfCompositionSink {
  ULONG refs = 1;
  ITfContext *context;

public:
  bool called = false, accepted = false;
  HRESULT result = E_UNEXPECTED;
  explicit RefusalProbe(ITfContext *value) : context(value) {
    refusalProbeAlive = true;
  }
  ~RefusalProbe() { refusalProbeAlive = false; }
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void **out) override {
    if (!out)
      return E_POINTER;
    *out = nullptr;
    if (id == IID_IUnknown || id == IID_ITfEditSession)
      *out = static_cast<ITfEditSession *>(this);
    else if (id == IID_ITfCompositionSink)
      *out = static_cast<ITfCompositionSink *>(this);
    if (!*out)
      return E_NOINTERFACE;
    AddRef();
    return S_OK;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++refs; }
  ULONG STDMETHODCALLTYPE Release() override {
    const auto count = --refs;
    if (!count)
      delete this;
    return count;
  }
  HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie cookie) override {
    called = true;
    TF_SELECTION selection{};
    ULONG fetched = 0;
    result = context->GetSelection(cookie, TF_DEFAULT_SELECTION, 1, &selection,
                                   &fetched);
    ITfContextComposition *owner = nullptr;
    ITfComposition *composition = nullptr;
    if (SUCCEEDED(result))
      result = context->QueryInterface(IID_ITfContextComposition,
                                       reinterpret_cast<void **>(&owner));
    if (SUCCEEDED(result) && (fetched != 1 || !selection.range))
      result = E_FAIL;
    if (SUCCEEDED(result))
      result =
          owner->StartComposition(cookie, selection.range, this, &composition);
    accepted = composition != nullptr;
    if (composition) {
      composition->EndComposition(cookie);
      composition->Release();
    }
    if (owner)
      owner->Release();
    if (selection.range)
      selection.range->Release();
    return result;
  }
  HRESULT STDMETHODCALLTYPE OnCompositionTerminated(TfEditCookie,
                                                    ITfComposition *) override {
    return S_OK;
  }
};
int run(int argc, char **argv) {
  require(argc == 3 || argc == 4,
          "Usage: test-tsf dll euc-jp-dictionary "
          "[--rejection|--rejection-control|--direct-key|--ai|--reentrant-"
          "scope]");
  const std::string mode = argc == 4 ? argv[3] : "";
#ifndef __MINGW32__
  require(conversionCompartmentId ==
                  GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION &&
              languageBarId == GUID_LBI_INPUTMODE &&
              systemTrayCategory == GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
          "TSF compatibility GUIDs differ from Windows SDK");
#endif
  const bool directKey = mode == "--direct-key";
  const bool reentrant = mode.starts_with("--reentrant-");
  SetEnvironmentVariableA("TGSKK_DICTIONARY", argv[2]);
  require(SUCCEEDED(CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED)),
          "COM init");
  HMODULE dll = LoadLibraryA(argv[1]);
  require(dll != nullptr, "LoadLibrary");
  auto getFactory =
      reinterpret_cast<HRESULT(WINAPI *)(REFCLSID, REFIID, void **)>(
          GetProcAddress(dll, "DllGetClassObject"));
  auto registration = reinterpret_cast<HRESULT(WINAPI *)()>(
      GetProcAddress(dll, "DllRegisterServer"));
  auto unregistration = reinterpret_cast<HRESULT(WINAPI *)()>(
      GetProcAddress(dll, "DllUnregisterServer"));
  auto unload = reinterpret_cast<HRESULT(WINAPI *)()>(
      GetProcAddress(dll, "DllCanUnloadNow"));
  RegistrationGuard cleanup{unregistration};
  require(getFactory && unload, "COM exports");

  IClassFactory *factory = nullptr;
  require(SUCCEEDED(getFactory(serviceId, IID_IClassFactory,
                               reinterpret_cast<void **>(&factory))),
          "factory");
  ITfTextInputProcessorEx *service = nullptr;
  require(
      SUCCEEDED(factory->CreateInstance(nullptr, IID_ITfTextInputProcessorEx,
                                        reinterpret_cast<void **>(&service))),
      "service");
  factory->Release();
  service->Release();
  service = nullptr;
  require(unload() == S_OK, "manual COM ownership leaked");
  require(registration && unregistration, "registration exports");
  const bool externalRegistration =
      GetEnvironmentVariableW(L"TGSKK_TEST_EXTERNAL_REGISTRATION", nullptr, 0) >
      0;
  if (!externalRegistration) {
    require(SUCCEEDED(registration()), "TSF registration");
    cleanup.active = true;
    require(registration() == S_OK, "repeated TSF registration");
  }
  ITfThreadMgr *manager = nullptr;
  require(SUCCEEDED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr,
                                     CLSCTX_INPROC_SERVER, IID_ITfThreadMgr,
                                     reinterpret_cast<void **>(&manager))),
          "TSF manager");
  TfClientId id = 0;
  require(SUCCEEDED(manager->Activate(&id)), "TSF activation");
  ITfCompartmentMgr *controlManager = nullptr;
  require(
      SUCCEEDED(manager->QueryInterface(
          IID_ITfCompartmentMgr, reinterpret_cast<void **>(&controlManager))),
      "IME compartment manager");
  auto setControl = [&](REFGUID guid, LONG value, bool dispatch = true) {
    ITfCompartment *control = nullptr;
    require(SUCCEEDED(controlManager->GetCompartment(guid, &control)),
            "IME compartment");
    VARIANT data;
    VariantInit(&data);
    data.vt = VT_I4;
    data.lVal = value;
    const auto result = control->SetValue(id, &data);
    control->Release();
    require(SUCCEEDED(result), "set IME compartment");
    if (dispatch)
      pump();
  };
  ITfDocumentMgr *document = nullptr;
  require(SUCCEEDED(manager->CreateDocumentMgr(&document)), "document");
  ITfDocumentMgr *emptyDocument = nullptr;
  require(SUCCEEDED(manager->CreateDocumentMgr(&emptyDocument)),
          "unfocused document");
  auto *store = new Store;
  store->window =
      CreateWindowW(L"STATIC", L"tgskk test", WS_OVERLAPPEDWINDOW, 0, 0, 800,
                    600, nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
  require(store->window != nullptr, "test window");
  ShowWindow(store->window, SW_SHOW);
  SetForegroundWindow(store->window);
  SetFocus(store->window);
  ITfContext *context = nullptr;
  TfEditCookie cookie;
  require(SUCCEEDED(document->CreateContext(
              id, 0, static_cast<ITextStoreACP *>(store), &context, &cookie)),
          "context");
  require(SUCCEEDED(document->Push(context)), "push context");
  require(SUCCEEDED(manager->SetFocus(document)), "focus document");
  ITfDocumentMgr *previous = nullptr;
  require(
      SUCCEEDED(manager->AssociateFocus(store->window, document, &previous)),
      "associate window focus");
  CaptureRegistration captured;
  if (directKey || reentrant) {
    require(SUCCEEDED(getFactory(serviceId, IID_IClassFactory,
                                 reinterpret_cast<void **>(&factory))),
            "capturing factory target");
    captured.factory.reset(new CapturingFactory(factory));
    DWORD registrationCookie = 0;
    const auto registered = CoRegisterClassObject(
        serviceId, captured.factory.get(), CLSCTX_INPROC_SERVER,
        REGCLS_MULTIPLEUSE, &registrationCookie);
    captured.cookie = registrationCookie;
    require(SUCCEEDED(registered), "capturing factory registration");
  }

  ITfInputProcessorProfiles *profiles = nullptr;
  require(SUCCEEDED(CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr,
                                     CLSCTX_INPROC_SERVER,
                                     IID_ITfInputProcessorProfiles,
                                     reinterpret_cast<void **>(&profiles))),
          "profiles");
  LANGID language = 0;
  require(SUCCEEDED(profiles->GetCurrentLanguage(&language)),
          "current input language");
  const LANGID japanese = MAKELANGID(LANG_JAPANESE, SUBLANG_DEFAULT);
  BSTR description = nullptr;
  require(SUCCEEDED(profiles->GetLanguageProfileDescription(
              serviceId, japanese, profileId, &description)),
          "Japanese profile registration");
  require(description && std::wstring(description) == L"tgskk",
          "Japanese profile description");
  SysFreeString(description);
  // Hosted Windows runners lack Japanese language support. Route the same TIP
  // through a temporary profile in the runner's installed input language.
  if (language != japanese && !externalRegistration)
    require(SUCCEEDED(profiles->AddLanguageProfile(serviceId, language,
                                                   profileId, L"tgskk test", 10,
                                                   nullptr, 0, 0)),
            "temporary test profile");
  require(SUCCEEDED(profiles->EnableLanguageProfile(serviceId, language,
                                                    profileId, TRUE)),
          "enable profile");
  const auto activated =
      profiles->ActivateLanguageProfile(serviceId, language, profileId);
  if (FAILED(activated))
    std::fprintf(stderr, "TSF profile activation HRESULT=%08lx lang=%04x\n",
                 activated, language);
  require(SUCCEEDED(activated), "profile activation");
  pump();
  require(SUCCEEDED(manager->SetFocus(document)), "focus activated document");
  pump();
  // Profile activation restores Windows IME state; set the fixture afterwards.
  setControl(GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, 1);
  setControl(conversionCompartmentId, IME_CMODE_NATIVE | IME_CMODE_FULLSHAPE);
  ITfKeystrokeMgr *keys = nullptr;
  require(SUCCEEDED(manager->QueryInterface(IID_ITfKeystrokeMgr,
                                            reinterpret_cast<void **>(&keys))),
          "key routing");
  if (directKey) {
    ITfKeyEventSink *direct = captured.factory->sink;
    require(direct != nullptr, "capture active TSF key sink");
    BYTE controlState[256] = {};
    controlState[VK_CONTROL] = controlState[VK_LCONTROL] = 0x80;
    require(SetKeyboardState(controlState), "direct control key state");
    const LPARAM controlScan = 1 | (MapVirtualKeyW('C', MAPVK_VK_TO_VSC) << 16);
    BOOL eligible = TRUE, consumed = TRUE;
    require(SUCCEEDED(
                direct->OnTestKeyDown(context, 'C', controlScan, &eligible)) &&
                !eligible,
            "Ctrl+C was eligible");
    const auto beforeShortcut = store->startedCompositions;
    store->refuseComposition = true;
    require(
        SUCCEEDED(direct->OnKeyDown(context, 'C', controlScan, &consumed)) &&
            !consumed,
        "direct Ctrl+C was consumed");
    require(store->startedCompositions == beforeShortcut,
            "unhandled key started a composition");
    require(store->text.empty(), "unhandled key changed document");
    store->refuseComposition = false;
    idleEditingKeys(direct, context, store);
    require(SUCCEEDED(captured.close()), "revoke capturing factory");
    BYTE clearState[256] = {};
    require(SetKeyboardState(clearState), "restore key state");
  }
  auto key = [&](UINT code, bool shift = false, bool win = false,
                 bool control = false) {
    BYTE state[256] = {};
    if (shift)
      state[VK_SHIFT] = state[VK_LSHIFT] = 0x80;
    if (win)
      state[VK_LWIN] = 0x80;
    if (control)
      state[VK_CONTROL] = state[VK_LCONTROL] = 0x80;
    require(SetKeyboardState(state), "keyboard state");
    BOOL test = FALSE, eaten = FALSE;
    const LPARAM scan = 1 | (MapVirtualKeyW(code, MAPVK_VK_TO_VSC) << 16);
    const HRESULT testResult = keys->TestKeyDown(code, scan, &test);
    require(SUCCEEDED(testResult), "key eligibility");
    if (test)
      require(SUCCEEDED(keys->KeyDown(code, scan, &eaten)), "key callback");
    if (!eaten) {
      CLSID foreground{};
      HRESULT foregroundResult = keys->GetForeground(&foreground);
      LANGID activeLanguage = 0;
      GUID activeProfile{};
      HRESULT activeResult = profiles->GetActiveLanguageProfile(
          serviceId, &activeLanguage, &activeProfile);
      std::fprintf(stderr,
                   "key=%u test=%08lx eligible=%d foreground=%08lx ours=%d "
                   "profile=%08lx lang=%04x ours=%d focus=%p/%p\n",
                   code, testResult, test, foregroundResult,
                   foreground == serviceId, activeResult, activeLanguage,
                   activeProfile == profileId, GetFocus(), store->window);
    }
    require(testResult == S_OK, "TSF key sink unavailable");
    pump();
    return eaten;
  };
  auto terminate = [&](bool required) {
    ITfContextComposition *compositions = nullptr;
    require(SUCCEEDED(context->QueryInterface(
                IID_ITfContextComposition,
                reinterpret_cast<void **>(&compositions))),
            "context compositions");
    IEnumITfCompositionView *items = nullptr;
    require(SUCCEEDED(compositions->EnumCompositions(&items)),
            "enumerate compositions");
    ITfCompositionView *view = nullptr;
    ULONG fetched = 0;
    require(SUCCEEDED(items->Next(1, &view, &fetched)), "composition view");
    require(!required || fetched == 1, "no composition to terminate");
    if (view) {
      ITfContextOwnerCompositionServices *owner = nullptr;
      require(SUCCEEDED(context->QueryInterface(
                  IID_ITfContextOwnerCompositionServices,
                  reinterpret_cast<void **>(&owner))),
              "owner composition services");
      require(SUCCEEDED(owner->TerminateComposition(view)),
              "forced composition termination");
      owner->Release();
      view->Release();
    }
    items->Release();
    compositions->Release();
    pump();
  };
  if (mode == "--rejection-control") {
    store->refuseComposition = true;
    auto *probe = new RefusalProbe(context);
    HRESULT status = E_FAIL;
    require(SUCCEEDED(context->RequestEditSession(
                id, probe, TF_ES_ASYNCDONTCARE | TF_ES_READWRITE, &status)),
            "standalone refusal request");
    pump();
    require(probe->called && probe->result == S_OK && !probe->accepted,
            "standalone refusal result");
    require(probe->Release() <= 1, "extra standalone refusal references");
    store->refuseComposition = false;
  } else if (mode == "--rejection") {
    store->refuseComposition = true;
    require(!key('K', true), "rejected composition consumed key");
    require(store->text.empty(), "rejected composition changed document");
    store->refuseComposition = false;
    require(key('A'), "input after rejected composition");
    require(store->text == L"あ", "rejected composition advanced input state");
  } else if (reentrant) {
    auto *direct = captured.factory->sink;
    require(direct != nullptr, "capture reentrant service");
    ITfTextInputProcessor *processor = nullptr;
    require(
        SUCCEEDED(direct->QueryInterface(
            IID_ITfTextInputProcessor, reinterpret_cast<void **>(&processor))),
        "reentrant processor");
    store->replace(L"駅で");
    pump();
    bool called = false;
    HRESULT deactivated = E_FAIL;
    auto callback = [&] {
      called = true;
      std::puts("Host scope callback entered.");
      std::fflush(stdout);
      deactivated = processor->Deactivate();
    };
    *store->scopeRead = callback;
    const bool consumed = key('K', true);
    *store->scopeRead = {};
    require(!consumed, "invalidated service consumed key");
    require(called && SUCCEEDED(deactivated),
            "read callback did not deactivate service");
    require(store->text == L"駅で", "reentrant deactivation changed document");
    processor->Release();
    require(SUCCEEDED(captured.close()), "close reentrant factory");
    std::puts("Deactivation during host reads leaves document unchanged.");
  } else if (mode == "--ai") {
    for (bool missingScope : {false, true}) {
      store->setScope(IS_DEFAULT, missingScope);
      auto convert = [&](const std::wstring &prefix) {
        store->replace(prefix);
        pump();
        require(key('K', true) && key('I') && key('S') && key('H') &&
                    key('A') && key(VK_SPACE),
                "contextual conversion keys");
      };
      bool train = false;
      for (unsigned attempt = 0; attempt < 5; ++attempt) {
        convert(L"駅で");
        train = store->text == L"駅で▼汽車";
        if (train)
          break;
        terminate(true);
      }
      require(train, "native TSF did not use left context for train ranking");
      require(key(VK_RETURN), "commit contextual train");
      require(store->text == L"駅で汽車", "contextual train commit");
      convert(L"新聞の");
      require(store->text == L"新聞の▼記者",
              "native TSF retained stale train context");
      require(key(VK_RETURN), "commit contextual reporter");
      require(store->text == L"新聞の記者", "contextual reporter commit");
      std::puts("Native TSF surrounding-text AI ranking and commit passed.");
    }
  } else if (mode.empty()) {
    ITfLangBarItemMgr *barManager = nullptr;
    require(SUCCEEDED(manager->QueryInterface(
                IID_ITfLangBarItemMgr, reinterpret_cast<void **>(&barManager))),
            "language bar manager");
    ITfLangBarItem *bar = nullptr;
    require(SUCCEEDED(barManager->GetItem(languageBarId, &bar)),
            "language bar item");
    barManager->Release();
    ITfLangBarItemButton *button = nullptr;
    require(SUCCEEDED(bar->QueryInterface(IID_ITfLangBarItemButton,
                                          reinterpret_cast<void **>(&button))),
            "language bar button");
    bar->Release();
    BOOL preserved = FALSE;
    require(
        SUCCEEDED(keys->SimulatePreservedKey(context, imeOffId, &preserved)) &&
            preserved,
        "preserved IME OFF");
    require(!key('A') && store->text.empty(), "IME OFF consumed input");
    BSTR label = nullptr;
    require(SUCCEEDED(button->GetText(&label)) && std::wstring(label) == L"OFF",
            "OFF label");
    SysFreeString(label);
    HICON icon = nullptr;
    require(SUCCEEDED(button->GetIcon(&icon)) && icon, "mode icon");
    DestroyIcon(icon);
    require(
        SUCCEEDED(keys->SimulatePreservedKey(context, imeOnId, &preserved)) &&
            preserved,
        "preserved IME ON");
    require(SUCCEEDED(button->OnMenuSelect(2)) && key('A') &&
                store->text == L"ア",
            "language bar katakana");
    store->replace(L"");
    pump();
    setControl(conversionCompartmentId, 0);
    require(!key('A'), "OS ASCII mode was ignored");
    require(SUCCEEDED(button->OnMenuSelect(1)), "language bar hiragana");
    require(key('K', true) && key('A'), "compose before IME OFF");
    store->holdAsync = true;
    setControl(GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, 0);
    require(key('A') && key('I') && store->text == L"▽か",
            "IME OFF forwarded keys before deferred cleanup");
    store->grantAsync();
    for (unsigned attempt = 0; attempt < 25 && !store->text.empty(); ++attempt)
      pump();
    require(store->text.empty() && !key('A'),
            "IME OFF retained pending composition");
    setControl(GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, 1);
    button->Release();
    *store->scopeRead = [&] {
      setControl(GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, 0, false);
    };
    require(!key('K', true) && store->text.empty(),
            "reentrant IME OFF consumed key");
    *store->scopeRead = {};
    setControl(GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, 1);
    setControl(conversionCompartmentId,
               IME_CMODE_ROMAN | IME_CMODE_NATIVE | IME_CMODE_FULLSHAPE);
    require(key('L'), "enter ASCII mode");
    const HWND popup = findModePopup(L"A");
    require(popup && IsWindowVisible(popup), "mode popup was not shown");
    const ULONGLONG popupDeadline = GetTickCount64() + 5000;
    bool hidden = false;
    while (!hidden && GetTickCount64() < popupDeadline) {
      pump();
      hidden = !IsWindowVisible(popup);
    }
    require(hidden, "mode popup ignored its timer");
    ITfCompartment *conversion = nullptr;
    require(SUCCEEDED(controlManager->GetCompartment(conversionCompartmentId,
                                                     &conversion)),
            "conversion compartment for publication");
    VARIANT published;
    VariantInit(&published);
    require(SUCCEEDED(conversion->GetValue(&published)) &&
                published.vt == VT_I4 && published.lVal == IME_CMODE_ROMAN,
            "engine mode was not published or discarded OS flags");
    VariantClear(&published);
    conversion->Release();
    const auto compositionsBeforeAscii = store->startedCompositions;
    require(!key('A') && !key('A', true) && !key(VK_SPACE),
            "ASCII keys were consumed");
    require(store->text.empty() &&
                store->startedCompositions == compositionsBeforeAscii,
            "ASCII keys started a composition");
    store->appInsert(L"aA ");
    require(key('J', false, false, true), "return from ASCII mode");
    const HWND kanaPopup = findModePopup(L"あ");
    require(kanaPopup && IsWindowVisible(kanaPopup),
            "kana mode popup was not shown");
    require(key('A') && store->text == L"aA あ",
            "kana after ASCII passthrough");
    require(!IsWindowVisible(kanaPopup), "mode popup stayed visible");
    store->textExt = {0, 0, 0, 0};
    require(key('L'), "ASCII mode with an empty caret rect");
    require(!IsWindowVisible(popup), "mode popup ignored an empty caret rect");
    store->textExt = {0, 0, 10, 20};
    setControl(conversionCompartmentId,
               IME_CMODE_ROMAN | IME_CMODE_NATIVE | IME_CMODE_FULLSHAPE);
    *store->textExtRead = [&] {
      setControl(GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, 0, false);
    };
    require(key('L'), "ASCII mode while the input mode turns off");
    require(!IsWindowVisible(popup),
            "mode popup was shown after popups were hidden");
    setControl(GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, 1);
    setControl(conversionCompartmentId,
               IME_CMODE_ROMAN | IME_CMODE_NATIVE | IME_CMODE_FULLSHAPE);
    store->replace(L"");
    pump();
    require(key('K', true), "uppercase kana rejected");
    require(key('A'), "kana continuation");
    require(store->text == L"▽か", "wrong preedit");
    key(VK_ESCAPE);
    require(SUCCEEDED(manager->SetFocus(emptyDocument)), "leave document");
    pump();
    require(SUCCEEDED(manager->SetFocus(document)), "restore document");
    pump();
    require(store->text.empty(), "focus retained preedit");
    require(!key('K', false, true), "Win shortcut consumed");
    require(store->text.empty(), "Win shortcut changed document");
    for (auto scope :
         {IS_PASSWORD, IS_PRIVATE, IS_NUMERIC_PASSWORD, IS_NUMERIC_PIN,
          IS_ALPHANUMERIC_PIN, IS_ALPHANUMERIC_PIN_SET}) {
      store->setScope(scope);
      pump();
      require(!key('K', true), "sensitive field consumed input");
      require(store->text.empty(), "sensitive field changed document");
    }
    store->setScope(IS_DEFAULT, true);
    pump();
    require(key('A'), "unavailable field scope blocked local input");
    require(store->text == L"あ", "unavailable field scope lost local input");
    store->replace(L"");
    pump();
    store->setScope(IS_DEFAULT);
    pump();

    require(key('K', true) && key('A'), "start before forced termination");
    terminate(true);
    require(store->text.empty(), "forced termination retained preedit");
    require(key('K', true) && key('A'), "start before external edit");
    store->replace(L"外部");
    pump();
    terminate(false);
    require(store->text == L"外部", "termination erased external edits");
    store->replace(L"");
    pump();

    require(key('K', true) && key('O') && key('U') && key(VK_SPACE),
            "candidate conversion");
    require(store->text == L"▼候補01", "first inline candidate");
    require(key(VK_NEXT), "inline Page_Down propagated");
    require(store->text == L"▼候補05", "first candidate page");
    require(key(VK_NEXT), "list Page_Down propagated");
    require(store->text == L"▼候補12", "next candidate page");
    require(key(VK_NEXT), "last-page Page_Down propagated");
    require(store->text == L"▼候補12", "Page_Down passed last page");
    require(key(VK_PRIOR), "list Page_Up propagated");
    require(store->text == L"▼候補05", "previous candidate page");
    require(key('X'), "candidate page previous");
    require(store->text == L"▼候補04", "return to inline candidate");
    require(key(VK_PRIOR), "inline Page_Up propagated");
    require(store->text == L"▼候補04", "inline Page_Up changed candidate");
    require(key(VK_DOWN) && key('3'), "numeric candidate selection");
    require(store->text == L"候補07",
            "numeric selection chose wrong candidate");
    store->replace(L"");
    pump();

    store->denyLock = true;
    require(!key('K', true), "unavailable edit lock consumed key");
    require(store->text.empty(), "unavailable edit lock changed document");
    store->denyLock = false;
    require(key('A'), "start after unavailable edit lock");
    require(store->text == L"あ", "unavailable edit lock advanced input state");
    require(key('K', true) && key('A'), "start before caret movement");
    store->move(0);
    pump();
    require(store->text == L"あ", "caret movement retained stale preedit");
    require(key('I'), "input after caret movement");
    require(store->text == L"いあ", "input used old caret position");
    store->replace(L"");
    pump();
    require(key('K', true) && key('A'), "start before delayed cleanup");
    store->holdAsync = true;
    require(SUCCEEDED(manager->SetFocus(emptyDocument)), "deferred focus loss");
    pump();
    require(SUCCEEDED(manager->SetFocus(document)), "deferred focus restore");
    const bool resumed = key('I');
    if (!resumed) {
      store->appInsert(L"i");
      pump();
    }
    const auto afterInput = store->text;
    require(afterInput == (resumed ? L"い" : L"▽かi"),
            "pending cleanup lost or duplicated input");
    store->grantAsync();
    pump();
    require(store->text == afterInput, "deferred cleanup erased later input");
    store->replace(L"");
    pump();
    require(key('K', true), "start after refusal");
    store->failSelection = true;
    require(key('A'), "post-insertion failure forwarded key twice");
    store->failSelection = false;
    pump();
    require(store->text == L"▽か",
            "post-insertion failure lost or duplicated input");
    store->replace(L"");
    pump();
    require(key('K', true) && key('A'),
            "start before cleanup selection failure");
    require(store->text == L"▽か",
            "wrong preedit before cleanup selection failure");
    const ULONG beforeCleanup = store->endedCompositions;
    store->failSelectionAfterWrite = true;
    require(SUCCEEDED(manager->SetFocus(emptyDocument)),
            "cleanup selection failure focus");
    pump();
    require(!store->failSelectionAfterWrite && store->failSelection,
            "cleanup did not reach the post-write selection fault");
    require(store->endedCompositions == beforeCleanup + 1,
            "selection failure prevented composition termination");
    if (!store->text.empty()) {
      std::fprintf(stderr,
                   "cleanup: write_pending=%d selection_failure=%d text=",
                   store->failSelectionAfterWrite, store->failSelection);
      for (wchar_t ch : store->text)
        std::fprintf(stderr, "%04x ", static_cast<unsigned>(ch));
      std::fputc('\n', stderr);
    }
    store->failSelection = false;
    require(store->text.empty(), "cleanup selection failure retained preedit");
    require(SUCCEEDED(manager->SetFocus(document)),
            "focus after cleanup selection failure");
    require(key('I'), "input after cleanup selection failure");
    require(store->text == L"い",
            "cleanup selection failure retained input state");
  }
  require(SUCCEEDED(manager->SetFocus(emptyDocument)), "final focus loss");
  pump();
  ITfContextComposition *remaining = nullptr;
  require(
      SUCCEEDED(context->QueryInterface(IID_ITfContextComposition,
                                        reinterpret_cast<void **>(&remaining))),
      "final compositions");
  IEnumITfCompositionView *items = nullptr;
  require(SUCCEEDED(remaining->EnumCompositions(&items)),
          "final composition enumeration");
  ITfCompositionView *leftover = nullptr;
  ULONG found = 0;
  require(SUCCEEDED(items->Next(1, &leftover, &found)),
          "final composition count");
  if (leftover)
    leftover->Release();
  items->Release();
  remaining->Release();
  require(found == 0, "composition remained after final commit");
  controlManager->Release();
  keys->Release();
  ITfInputProcessorProfileMgr *profileManager = nullptr;
  require(SUCCEEDED(profiles->QueryInterface(
              IID_ITfInputProcessorProfileMgr,
              reinterpret_cast<void **>(&profileManager))),
          "profile manager");
  require(SUCCEEDED(profileManager->DeactivateProfile(
              TF_PROFILETYPE_INPUTPROCESSOR, language, serviceId, profileId,
              nullptr, TF_IPPMF_FORPROCESS)),
          "profile deactivation");
  profileManager->Release();
  pump();
  profiles->Release();
  ITfDocumentMgr *replaced = nullptr;
  require(
      SUCCEEDED(manager->AssociateFocus(store->window, previous, &replaced)),
      "restore window association");
  if (replaced)
    replaced->Release();
  if (previous)
    previous->Release();
  require(SUCCEEDED(document->Pop(TF_POPF_ALL)), "pop document contexts");
  std::fprintf(stderr, "test context released references=%lu\n",
               context->Release());
  document->Release();
  emptyDocument->Release();
  require(SUCCEEDED(manager->Deactivate()), "deactivate thread manager");
  manager->Release();
  DestroyWindow(store->window);
  std::fprintf(stderr, "test store released references=%lu\n",
               store->Release());
  pump();
  if (!externalRegistration) {
    require(SUCCEEDED(unregistration()), "TSF unregistration");
    require(unregistration() == S_OK, "repeated TSF unregistration");
    cleanup.active = false;
  }
  pump();
  CoFreeUnusedLibrariesEx(0, 0);
  CoUninitialize();
  FreeLibrary(dll);
  return 0;
}
int prepareFixture(HMODULE dll, bool remove) {
  auto registration = reinterpret_cast<HRESULT(WINAPI *)()>(GetProcAddress(
      dll, remove ? "DllUnregisterServer" : "DllRegisterServer"));
  require(registration != nullptr, "fixture registration export");
  require(SUCCEEDED(CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED)),
          "fixture COM");
  require(SUCCEEDED(registration()), "fixture registration");
  if (!remove) {
    ITfInputProcessorProfiles *profiles = nullptr;
    require(SUCCEEDED(CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr,
                                       CLSCTX_INPROC_SERVER,
                                       IID_ITfInputProcessorProfiles,
                                       reinterpret_cast<void **>(&profiles))),
            "fixture profiles");
    LANGID language = 0;
    require(SUCCEEDED(profiles->GetCurrentLanguage(&language)),
            "fixture language");
    if (language != MAKELANGID(LANG_JAPANESE, SUBLANG_DEFAULT))
      require(SUCCEEDED(profiles->AddLanguageProfile(serviceId, language,
                                                     profileId, L"tgskk test",
                                                     10, nullptr, 0, 0)),
              "fixture temporary profile");
    profiles->Release();
  }
  CoUninitialize();
  return 0;
}
int main(int argc, char **argv) {
  HMODULE dll = nullptr;
  try {
    require(argc == 3 || argc == 4,
            "Usage: test-tsf dll euc-jp-dictionary "
            "[--rejection|--rejection-control|--direct-key|--ai|--reentrant-"
            "scope]");
    require(argc == 3 || std::string(argv[3]) == "--rejection" ||
                std::string(argv[3]) == "--rejection-control" ||
                std::string(argv[3]) == "--direct-key" ||
                std::string(argv[3]) == "--ai" ||
                std::string(argv[3]) == "--reentrant-scope" ||
                std::string(argv[3]) == "--prepare-fixture" ||
                std::string(argv[3]) == "--remove-fixture",
            "unknown test mode");
    dll = LoadLibraryA(argv[1]);
    require(dll != nullptr, "retain test DLL");
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    if (argc == 4 && (std::string(argv[3]) == "--prepare-fixture" ||
                      std::string(argv[3]) == "--remove-fixture")) {
      const auto result =
          prepareFixture(dll, std::string(argv[3]) == "--remove-fixture");
      FreeLibrary(dll);
      return result;
    }
    auto unload = reinterpret_cast<HRESULT(WINAPI *)()>(
        GetProcAddress(dll, "DllCanUnloadNow"));
    require(unload != nullptr, "unload export");
    std::exception_ptr error;
    int result = 1;
    std::thread inputThread([&] {
      try {
        result = run(argc, argv);
      } catch (...) {
        error = std::current_exception();
      }
    });
    inputThread.join();
    if (error)
      std::rethrow_exception(error);
    const std::string mode = argc == 4 ? argv[3] : "";
    const auto unloadDeadline = GetTickCount64() + 4000;
    HRESULT unloaded;
    do {
      unloaded = unload();
      if (unloaded == S_OK)
        break;
      Sleep(10);
    } while (GetTickCount64() < unloadDeadline);
    if (mode == "--rejection" && unloaded == S_FALSE) {
      std::fputs("Windows retained the rejected composition sink; compare with "
                 "the independent control.\n",
                 stderr);
      result = 2;
    } else {
      require(unloaded == S_OK, "COM objects remain after input-thread exit");
      if (mode == "--rejection-control" && refusalProbeAlive)
        result = 2;
    }
    if (!result)
      std::puts("TSF input and COM lifetime checks passed.");
    if (unloaded == S_OK)
      FreeLibrary(dll);
    return result;
  } catch (const std::exception &error) {
    std::fprintf(stderr, "%s\n", error.what());
    return 1;
  }
}
