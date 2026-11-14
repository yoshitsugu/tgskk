#pragma once
#include <msctf.h>

#ifdef __MINGW32__
// MinGW headers omit this Windows SDK capability GUID.
inline const GUID GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT = {
    0xccf05dd7,
    0x4a87,
    0x11d7,
    {0xa6, 0xe2, 0x00, 0x06, 0x5b, 0x84, 0x43, 0x5c}};
#endif

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
#ifndef TF_IPPMF_FORPROCESS
#define TF_IPPMF_FORPROCESS 0x10000000
#endif

#include <ctfutb.h>
#include <ocidl.h>
#include <olectl.h>
#ifndef __MINGW32__
#include <ctffunc.h>
#endif
// MinGW omits these interfaces. Method order follows the Windows SDK ABI.
#ifndef __ITfMenu_INTERFACE_DEFINED__
#define __ITfMenu_INTERFACE_DEFINED__
struct ITfMenu : IUnknown {
  virtual HRESULT STDMETHODCALLTYPE AddMenuItem(UINT, DWORD, HBITMAP, HBITMAP,
                                                const WCHAR *, ULONG,
                                                ITfMenu **) = 0;
};
#endif
#ifndef __ITfLangBarItemButton_INTERFACE_DEFINED__
#define __ITfLangBarItemButton_INTERFACE_DEFINED__
enum TfLBIClick { TF_LBI_CLK_RIGHT = 1, TF_LBI_CLK_LEFT = 2 };
inline const IID IID_ITfLangBarItemButton = {
    0x28c7f1d0,
    0xde25,
    0x11d2,
    {0xaf, 0xdd, 0x00, 0x10, 0x5a, 0x27, 0x99, 0xb5}};
struct ITfLangBarItemButton : ITfLangBarItem {
  virtual HRESULT STDMETHODCALLTYPE OnClick(TfLBIClick, POINT,
                                            const RECT *) = 0;
  virtual HRESULT STDMETHODCALLTYPE InitMenu(ITfMenu *) = 0;
  virtual HRESULT STDMETHODCALLTYPE OnMenuSelect(UINT) = 0;
  virtual HRESULT STDMETHODCALLTYPE GetIcon(HICON *) = 0;
  virtual HRESULT STDMETHODCALLTYPE GetText(BSTR *) = 0;
};
#endif
#ifndef TF_LBI_STYLE_BTN_MENU
#define TF_LBI_STYLE_HIDDENSTATUSCONTROL 0x00000001
#define TF_LBI_STYLE_BTN_MENU 0x00020000
#define TF_LBI_STYLE_SHOWNINTRAY 0x00000002
#define TF_LBI_STATUS_HIDDEN 0x00000001
#define TF_LBI_STATUS_DISABLED 0x00000002
#define TF_LBI_ICON 0x00000001
#define TF_LBI_TEXT 0x00000002
#define TF_LBI_TOOLTIP 0x00000004
#define TF_LBI_STATUS 0x00010000
#define TF_LBMENUF_CHECKED 0x00000001
#endif
