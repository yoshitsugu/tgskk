// SPDX-License-Identifier: GPL-3.0-or-later
#include "../tsf_compat.h"
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <inputscope.h>
#include <msctf.h>
#include <stdexcept>
#include <string>
#include <textstor.h>
#include <windows.h>

static void require(bool condition, const char *reason) {
  if (!condition) {
    throw std::runtime_error(reason);
  }
}
class Scope final : public ITfInputScope {
  ULONG refs = 1;
  InputScope value;

public:
  explicit Scope(InputScope scope) : value(scope) {}
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
  bool holdAsync = false;
  InputScope scope = IS_DEFAULT;
  bool scopeFailure = false;
  HWND window = nullptr;
  ~Store() {
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
    if (holdAsync && !(flags & TS_LF_SYNC)) {
      pendingLock |= flags;
      *result = TS_S_ASYNC;
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
    if (SUCCEEDED(result))
      start = end = start + static_cast<LONG>(count);
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
      values[0].varValue.punkVal = new Scope(scope);
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
    *rect = {0, 0, 10, 20};
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
    *allowed = !refuseComposition;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnUpdateComposition(ITfCompositionView *,
                                                ITfRange *) override {
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnEndComposition(ITfCompositionView *) override {
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
struct RegistrationGuard {
  HRESULT(WINAPI *remove)();
  bool active = false;
  ~RegistrationGuard() {
    if (active && FAILED(remove()))
      std::fputs("TSF cleanup failed; run regsvr32 /u on the test DLL.\n",
                 stderr);
  }
};
int run(int argc, char **argv) {
  require(argc == 3, "Usage: test-tsf dll euc-jp-dictionary");
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
  CLSID serviceId = {0xc7993bd4,
                     0x1094,
                     0x4968,
                     {0xa3, 0x11, 0x3d, 0xb8, 0xa0, 0xe0, 0x17, 0x39}};
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
  require(registration && unregistration, "registration exports");
  require(SUCCEEDED(registration()), "TSF registration");
  cleanup.active = true;
  require(registration() == S_OK, "repeated TSF registration");
  ITfThreadMgr *manager = nullptr;
  require(SUCCEEDED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr,
                                     CLSCTX_INPROC_SERVER, IID_ITfThreadMgr,
                                     reinterpret_cast<void **>(&manager))),
          "TSF manager");
  TfClientId id = 0;
  require(SUCCEEDED(manager->Activate(&id)), "TSF activation");
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
  GUID profileId = {0x70c846ab,
                    0x9fb5,
                    0x43cb,
                    {0x94, 0xbb, 0x74, 0x83, 0xb9, 0xb2, 0xf6, 0xf8}};
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
  if (language != japanese)
    require(SUCCEEDED(profiles->AddLanguageProfile(serviceId, language,
                                                   profileId, L"tgskk test", 10,
                                                   nullptr, 0, 0)),
            "temporary test profile");
  require(SUCCEEDED(profiles->EnableLanguageProfile(serviceId, language,
                                                    profileId, TRUE)),
          "enable profile");
  require(SUCCEEDED(profiles->ActivateLanguageProfile(serviceId, language,
                                                      profileId)),
          "profile activation");
  pump();
  require(SUCCEEDED(manager->SetFocus(document)), "focus activated document");
  ITfKeystrokeMgr *keys = nullptr;
  require(SUCCEEDED(manager->QueryInterface(IID_ITfKeystrokeMgr,
                                            reinterpret_cast<void **>(&keys))),
          "key routing");
  auto key = [&](UINT code, bool shift = false, bool win = false) {
    BYTE state[256] = {};
    if (shift)
      state[VK_SHIFT] = state[VK_LSHIFT] = 0x80;
    if (win)
      state[VK_LWIN] = 0x80;
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
  require(key('K', true), "uppercase kana rejected");
  require(key('A'), "kana continuation");
  require(store->text == L"▽か", "wrong preedit");
  key(VK_ESCAPE);
  require(SUCCEEDED(manager->SetFocus(emptyDocument)), "leave document");
  pump();
  require(SUCCEEDED(manager->SetFocus(document)), "restore document");
  pump();
  require(store->text.empty(), "focus retained preedit");
  store->refuseComposition = true;
  require(!key('K', true), "rejected composition consumed key");
  require(store->text.empty(), "rejected composition changed document");
  store->refuseComposition = false;
  require(!key('K', false, true), "Win shortcut consumed");
  require(store->text.empty(), "Win shortcut changed document");
  for (auto scope : {IS_PASSWORD, IS_PRIVATE}) {
    store->setScope(scope);
    pump();
    require(!key('K', true), "sensitive field consumed input");
    require(store->text.empty(), "sensitive field changed document");
  }
  store->setScope(IS_DEFAULT, true);
  pump();
  require(!key('K', true), "unknown field scope consumed input");
  require(store->text.empty(), "unknown field scope changed document");
  store->setScope(IS_DEFAULT);
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
  require(store->text == L"候補07", "numeric selection chose wrong candidate");
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
  require(!key('I'), "new input crossed pending cleanup");
  require(store->text == L"▽か", "deferred cleanup edited before lock grant");
  store->grantAsync();
  pump();
  require(store->text.empty(), "deferred cleanup retained preedit");
  require(key('I'), "input after deferred cleanup");
  require(store->text == L"い", "deferred cleanup erased later input");
  store->replace(L"");
  pump();
  require(key('K', true), "start after refusal");
  store->failSelection = true;
  require(key('A'), "post-insertion failure forwarded key twice");
  store->failSelection = false;
  pump();
  require(store->text == L"▽か",
          "post-insertion failure lost or duplicated input");
  keys->Release();
  require(SUCCEEDED(manager->SetFocus(emptyDocument)), "final focus loss");
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
  document->Pop(TF_POPF_ALL);
  context->Release();
  document->Release();
  emptyDocument->Release();
  manager->Deactivate();
  manager->Release();
  DestroyWindow(store->window);
  store->Release();
  pump();
  require(SUCCEEDED(unregistration()), "TSF unregistration");
  require(unregistration() == S_OK, "repeated TSF unregistration");
  cleanup.active = false;
  pump();
  CoUninitialize();
  require(unload() == S_OK, "COM objects leaked");
  FreeLibrary(dll);
  std::puts("TSF composition, rejection, key propagation, focus and COM "
            "lifetime passed.");
  return 0;
}
int main(int argc, char **argv) {
  try {
    return run(argc, argv);
  } catch (const std::exception &error) {
    std::fprintf(stderr, "%s\n", error.what());
    return 1;
  }
}
