// SPDX-License-Identifier: GPL-3.0-or-later
#include "../input_scope.h"
#include <algorithm>
#include <msctf.h>
#include <cstdio>
#include <stdexcept>
#include <utility>
#include <vector>
using namespace tgskk::windows;
namespace {
unsigned live = 0, reads = 0;
void require(bool value, const char *message) {
  if (!value)
    throw std::runtime_error(message);
}
class Scope final : public ITfInputScope {
  ULONG refs = 1;
  std::vector<InputScope> values;
  HRESULT result;
  bool malformed, supported;

public:
  Scope(std::vector<InputScope> scopes, HRESULT status, bool invalid,
        bool valid)
      : values(std::move(scopes)), result(status), malformed(invalid),
        supported(valid) {
    ++live;
  }
  ~Scope() { --live; }
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void **out) override {
    *out = nullptr;
    if (id != IID_IUnknown && (id != IID_ITfInputScope || !supported))
      return E_NOINTERFACE;
    *out = static_cast<ITfInputScope *>(this);
    AddRef();
    return S_OK;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++refs; }
  ULONG STDMETHODCALLTYPE Release() override {
    const auto n = --refs;
    if (!n)
      delete this;
    return n;
  }
  HRESULT STDMETHODCALLTYPE GetInputScopes(InputScope **out,
                                           UINT *count) override {
    ++reads;
    *out = nullptr;
    *count = 0;
    if (FAILED(result))
      return result;
    *count = static_cast<UINT>(values.size());
    if (!values.empty() && !malformed) {
      *out = static_cast<InputScope *>(
          CoTaskMemAlloc(values.size() * sizeof(InputScope)));
      if (!*out)
        return E_OUTOFMEMORY;
      std::copy(values.begin(), values.end(), *out);
    }
    return result;
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
void objectCase(std::vector<InputScope> scopes, HRESULT objectStatus,
                bool malformed, bool supported, HRESULT expected, bool known,
                HRESULT valueStatus = S_OK) {
  VARIANT value;
  VariantInit(&value);
  value.vt = VT_UNKNOWN;
  value.punkVal =
      new Scope(std::move(scopes), objectStatus, malformed, supported);
  reads = 0;
  const auto policy = inputScopePolicy(valueStatus, value);
  VariantClear(&value);
  require(policy.status == expected && policy.known == known,
          "scope object policy");
  const bool optionalMissing =
      valueStatus == S_FALSE || valueStatus == E_FAIL ||
      valueStatus == E_NOTIMPL || valueStatus == E_NOINTERFACE;
  require(policy.localAi == (known || optionalMissing),
          "local AI scope policy");
  require(policy.networkAllowed(false) == known,
          "default network scope policy");
  require(policy.networkAllowed(true) == (known || optionalMissing),
          "configured network scope policy");
  require(live == 0, "scope object reference leak");
  if (FAILED(valueStatus))
    require(reads == 0, "failed metadata query inspected stale scope data");
}
} // namespace
int main() {
  try {
    VARIANT value;
    VariantInit(&value);
    for (HRESULT status : {S_FALSE, E_FAIL, E_NOTIMPL, E_NOINTERFACE}) {
      const auto policy = inputScopePolicy(status, value);
      require(policy.status == S_OK && !policy.known && policy.localAi,
              "unavailable metadata must allow only local AI");
      require(!policy.networkAllowed(false) && policy.networkAllowed(true),
              "unavailable metadata network opt-in");
      objectCase({IS_PASSWORD}, S_OK, false, true, S_OK, false, status);
    }
    for (HRESULT status : {E_ACCESSDENIED, E_OUTOFMEMORY, TF_E_DISCONNECTED}) {
      const auto policy = inputScopePolicy(status, value);
      require(policy.status == status && !policy.known && !policy.localAi,
              "operational metadata failure");
      require(!policy.networkAllowed(true),
              "network opt-in bypassed operational failure");
    }
    for (VARTYPE type : {VT_EMPTY, VT_NULL}) {
      value.vt = type;
      const auto policy = inputScopePolicy(S_OK, value);
      require(policy.status == S_OK && !policy.known && policy.localAi,
              "empty metadata must allow only local AI");
    }
    for (VARTYPE type : {VT_I4, VT_BSTR, VT_UNKNOWN}) {
      value.vt = type;
      require(FAILED(inputScopePolicy(S_OK, value).status),
              "malformed metadata value");
    }
    objectCase({IS_DEFAULT}, S_OK, false, true, S_OK, true);
    objectCase({IS_URL, IS_SEARCH}, S_OK, false, true, S_OK, true);
    objectCase({}, S_OK, false, true, S_OK, false);
    objectCase({IS_DEFAULT}, S_FALSE, false, true, S_FALSE, false);
    objectCase({static_cast<InputScope>(999)}, S_OK, false, true, S_OK, false);
    objectCase({IS_DEFAULT, static_cast<InputScope>(999)}, S_OK, false, true,
               S_OK, false);
    objectCase({IS_DEFAULT}, E_FAIL, false, true, E_FAIL, false);
    objectCase({IS_DEFAULT}, E_ACCESSDENIED, false, true, E_ACCESSDENIED,
               false);
    objectCase({IS_DEFAULT}, S_OK, true, true, E_UNEXPECTED, false);
    objectCase({IS_DEFAULT}, S_OK, false, false, E_NOINTERFACE, false);
    for (InputScope secret :
         {IS_PASSWORD, IS_PRIVATE, IS_NUMERIC_PASSWORD, IS_NUMERIC_PIN,
          IS_ALPHANUMERIC_PIN, IS_ALPHANUMERIC_PIN_SET}) {
      objectCase({secret}, S_OK, false, true, E_ACCESSDENIED, false);
      objectCase({IS_DEFAULT, secret}, S_OK, false, true, E_ACCESSDENIED,
                 false);
      objectCase({secret, IS_DEFAULT}, S_OK, false, true, E_ACCESSDENIED,
                 false);
      objectCase({static_cast<InputScope>(999), secret}, S_OK, false, true,
                 E_ACCESSDENIED, false);
    }
    std::puts("input-scope metadata availability, operational failures, "
              "malformed values, mixed scopes and secret fields passed");
    return 0;
  } catch (const std::exception &error) {
    std::fprintf(stderr, "input-scope: %s\n", error.what());
    return 1;
  }
}
