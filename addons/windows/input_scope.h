// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <inputscope.h>
#include <oleauto.h>

namespace tgskk::windows {
struct InputScopePolicy {
  HRESULT status = S_OK;
  bool known = false;
  bool localAi = false;
  bool networkAllowed(bool allowWithoutScope) const {
    return SUCCEEDED(status) && (known || (localAi && allowWithoutScope));
  }
};
inline InputScopePolicy unavailableInputScope(HRESULT status) {
  // Optional metadata is unavailable in some ordinary text stores. Other
  // failures (access, allocation, disconnection, etc.) must still abort edits.
  if (status == S_FALSE || status == E_FAIL || status == E_NOTIMPL ||
      status == E_NOINTERFACE)
    return {S_OK, false, true};
  return {FAILED(status) ? status : E_UNEXPECTED, false};
}
inline bool sensitiveInputScope(InputScope value) {
  return value == IS_PASSWORD || value == IS_PRIVATE ||
         value == IS_NUMERIC_PASSWORD || value == IS_NUMERIC_PIN ||
         value == IS_ALPHANUMERIC_PIN || value == IS_ALPHANUMERIC_PIN_SET;
}
inline InputScopePolicy inputScopePolicy(HRESULT status, const VARIANT &value) {
  if (FAILED(status))
    return unavailableInputScope(status);
  if (status == S_FALSE)
    return unavailableInputScope(status);
  if (value.vt == VT_EMPTY || value.vt == VT_NULL)
    return unavailableInputScope(S_FALSE);
  if (value.vt != VT_UNKNOWN || !value.punkVal)
    return {E_INVALIDARG, false};
  ITfInputScope *object = nullptr;
  status = value.punkVal->QueryInterface(IID_ITfInputScope,
                                         reinterpret_cast<void **>(&object));
  if (FAILED(status) || !object)
    return {FAILED(status) ? status : E_UNEXPECTED, false};
  InputScope *scopes = nullptr;
  UINT count = 0;
  status = object->GetInputScopes(&scopes, &count);
  object->Release();
  if (SUCCEEDED(status) && count && !scopes)
    status = E_UNEXPECTED;
  bool known = status == S_OK && count > 0;
  if (SUCCEEDED(status)) {
    for (UINT i = 0; i < count; ++i) {
      if (sensitiveInputScope(scopes[i])) {
        status = E_ACCESSDENIED;
        known = false;
        break;
      }
      // New or invalid scope values are not evidence of a non-private field.
      known &=
          (scopes[i] >= IS_DEFAULT && scopes[i] <= IS_CHAT_WITHOUT_EMOJI) ||
          (scopes[i] >= IS_ENUMSTRING && scopes[i] <= IS_PHRASELIST);
    }
  }
  CoTaskMemFree(scopes);
  // Missing optional metadata permits local inference, but an explicit unknown
  // descriptor does not. External lookups without metadata require opt-in.
  return {status, known, known};
}
} // namespace tgskk::windows
