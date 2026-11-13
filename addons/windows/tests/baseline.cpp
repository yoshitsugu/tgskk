// SPDX-License-Identifier: GPL-3.0-or-later
// Reuse registration and DLL bookkeeping, replacing only the COM factory.
#define DllGetClassObject ProductGetClassObject
#include "../tsf.cpp"
#undef DllGetClassObject

namespace {
class BaselineService final : public ITfTextInputProcessorEx {
  std::atomic<ULONG> refs{1};

public:
  BaselineService() { traceCount("baseline created", ++objects); }
  ~BaselineService() { traceCount("baseline destroyed", --objects); }
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void **out) override {
    if (!out)
      return E_POINTER;
    *out = nullptr;
    if (id != IID_IUnknown && id != IID_ITfTextInputProcessor &&
        id != IID_ITfTextInputProcessorEx)
      return E_NOINTERFACE;
    *out = static_cast<ITfTextInputProcessorEx *>(this);
    AddRef();
    return S_OK;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++refs; }
  ULONG STDMETHODCALLTYPE Release() override {
    const auto count = --refs;
    traceCount("baseline released", count, "references");
    if (!count)
      delete this;
    return count;
  }
  HRESULT STDMETHODCALLTYPE Activate(ITfThreadMgr *, TfClientId) override {
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE ActivateEx(ITfThreadMgr *, TfClientId,
                                       DWORD) override {
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE Deactivate() override { return S_OK; }
};
class BaselineFactory final : public IClassFactory {
  std::atomic<ULONG> refs{1};

public:
  BaselineFactory() { ++objects; }
  ~BaselineFactory() { --objects; }
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
    if (!out)
      return E_POINTER;
    *out = nullptr;
    if (outer)
      return CLASS_E_NOAGGREGATION;
    auto *service = new (std::nothrow) BaselineService;
    if (!service)
      return E_OUTOFMEMORY;
    const auto result = service->QueryInterface(id, out);
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
} // namespace
extern "C" HRESULT __stdcall DllGetClassObject(REFCLSID id, REFIID iid,
                                               void **out) {
  if (!out)
    return E_POINTER;
  *out = nullptr;
  if (id != serviceId)
    return CLASS_E_CLASSNOTAVAILABLE;
  auto *factory = new (std::nothrow) BaselineFactory;
  if (!factory)
    return E_OUTOFMEMORY;
  const auto result = factory->QueryInterface(iid, out);
  factory->Release();
  return result;
}
