// SPDX-License-Identifier: GPL-3.0-or-later
#include "../tsf_compat.h"
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <msctf.h>
#include <string>
#include <textstor.h>
#include <windows.h>

static void require(bool condition, const char *reason) {
  if (!condition) {
    std::fprintf(stderr, "%s (error=%lu)\n", reason, GetLastError());
    std::exit(1);
  }
}
class Store final : public ITextStoreACP,
                    public ITfContextOwnerCompositionSink {
  ULONG refs = 1;
  ITextStoreACPSink *sink = nullptr;
  bool locked = false;
  DWORD pendingLock = 0;

public:
  std::wstring text;
  LONG start = 0, end = 0;
  bool refuseComposition = false, failSelection = false, denyLock = false;
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
    while (pendingLock) {
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
  HRESULT STDMETHODCALLTYPE RequestSupportedAttrs(DWORD, ULONG,
                                                  const TS_ATTRID *) override {
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE RequestAttrsAtPosition(LONG, ULONG,
                                                   const TS_ATTRID *,
                                                   DWORD) override {
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
  HRESULT STDMETHODCALLTYPE RetrieveRequestedAttrs(ULONG, TS_ATTRVAL *,
                                                   ULONG *count) override {
    *count = 0;
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
int main(int argc, char **argv) {
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
  require(registration && unregistration && SUCCEEDED(registration()),
          "TSF registration");
  ITfThreadMgr *manager = nullptr;
  require(SUCCEEDED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr,
                                     CLSCTX_INPROC_SERVER, IID_ITfThreadMgr,
                                     reinterpret_cast<void **>(&manager))),
          "TSF manager");
  TfClientId id = 0;
  require(SUCCEEDED(manager->Activate(&id)), "TSF activation");
  ITfDocumentMgr *document = nullptr;
  require(SUCCEEDED(manager->CreateDocumentMgr(&document)), "document");
  auto *store = new Store;
  store->window =
      CreateWindowW(L"STATIC", L"tgskk test", WS_OVERLAPPEDWINDOW, 0, 0, 800,
                    600, nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
  ITfContext *context = nullptr;
  TfEditCookie cookie;
  require(SUCCEEDED(document->CreateContext(
              id, 0, static_cast<ITextStoreACP *>(store), &context, &cookie)),
          "context");
  require(SUCCEEDED(document->Push(context)), "push context");
  require(SUCCEEDED(manager->SetFocus(document)), "focus document");
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
  LANGID language = MAKELANGID(LANG_JAPANESE, SUBLANG_DEFAULT);
  require(SUCCEEDED(profiles->EnableLanguageProfile(serviceId, language,
                                                    profileId, TRUE)),
          "enable profile");
  require(SUCCEEDED(profiles->ChangeCurrentLanguage(language)),
          "Japanese language");
  require(SUCCEEDED(profiles->ActivateLanguageProfile(serviceId, language,
                                                      profileId)),
          "profile activation");
  pump();
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
    SetKeyboardState(state);
    BOOL test = FALSE, eaten = FALSE;
    keys->TestKeyDown(code, 0, &test);
    if (test)
      require(SUCCEEDED(keys->KeyDown(code, 0, &eaten)), "key callback");
    pump();
    return eaten;
  };
  require(key('K', true), "uppercase kana rejected");
  require(key('A'), "kana continuation");
  require(store->text == L"▽か", "wrong preedit");
  key(VK_ESCAPE);
  manager->SetFocus(nullptr);
  pump();
  manager->SetFocus(document);
  pump();
  require(store->text.empty(), "focus retained preedit");
  store->refuseComposition = true;
  require(!key('K', true), "rejected composition consumed key");
  require(store->text.empty(), "rejected composition changed document");
  store->refuseComposition = false;
  require(!key('K', false, true), "Win shortcut consumed");
  require(store->text.empty(), "Win shortcut changed document");
  require(key('K', true), "start after refusal");
  store->failSelection = true;
  require(key('A'), "post-insertion failure forwarded key twice");
  store->failSelection = false;
  pump();
  require(store->text == L"▽か",
          "post-insertion failure lost or duplicated input");
  keys->Release();
  manager->SetFocus(nullptr);
  pump();
  profiles->Release();
  document->Pop(TF_POPF_ALL);
  context->Release();
  document->Release();
  manager->Deactivate();
  manager->Release();
  DestroyWindow(store->window);
  store->Release();
  pump();
  require(SUCCEEDED(unregistration()), "TSF unregistration");
  pump();
  CoUninitialize();
  require(unload() == S_OK, "COM objects leaked");
  FreeLibrary(dll);
  std::puts("TSF composition, rejection, key propagation, focus and COM "
            "lifetime passed.");
}
