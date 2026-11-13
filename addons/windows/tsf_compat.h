#pragma once
#include <msctf.h>

// Older MinGW headers omit this interface; its ABI follows the Windows SDK.
#ifndef __ITfTextInputProcessorEx_INTERFACE_DEFINED__
#define __ITfTextInputProcessorEx_INTERFACE_DEFINED__
inline const IID IID_ITfTextInputProcessorEx = {
    0x6e4e2102,
    0xf9cd,
    0x433d,
    {0xb4, 0x96, 0x30, 0x3c, 0xe0, 0x3a, 0x65, 0x07}};
struct ITfTextInputProcessorEx : ITfTextInputProcessor {
  virtual HRESULT STDMETHODCALLTYPE ActivateEx(ITfThreadMgr *, TfClientId,
                                               DWORD) = 0;
};
#endif
#ifndef TF_TMAE_SECUREMODE
#define TF_TMAE_SECUREMODE 0x00000002
#endif
#ifndef TF_TMAE_UIELEMENTENABLEDONLY
#define TF_TMAE_UIELEMENTENABLEDONLY 0x00000004
#endif
#ifndef TF_CLIENTID_NULL
#define TF_CLIENTID_NULL ((TfClientId)0)
#endif
