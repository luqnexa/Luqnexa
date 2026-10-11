// SPDX-License-Identifier: AGPL-3.0-only
#pragma once
#include "common.h"
// Windows only accepts registered TIPs in its real keyboard dispatcher. Substitute
// that dispatcher to avoid machine registration; delegate document/lock APIs to TSF.
class TestManager final : public ITfThreadMgr,public ITfKeystrokeMgr {
    ULONG refs_=1; ComPtr<ITfThreadMgr> real_; ComPtr<ITfKeyEventSink> sink_;
public:
    explicit TestManager(ITfThreadMgr* manager):real_(manager) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** out) override {
        if(!out) return E_POINTER; *out=nullptr;
        if(iid==IID_IUnknown || iid==IID_ITfThreadMgr) *out=static_cast<ITfThreadMgr*>(this);
        else if(iid==IID_ITfKeystrokeMgr) *out=static_cast<ITfKeystrokeMgr*>(this);
        else return real_->QueryInterface(iid,out);
        AddRef(); return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++refs_; }
    ULONG STDMETHODCALLTYPE Release() override { ULONG n=--refs_; if(!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE Activate(TfClientId* id) override { return real_->Activate(id); }
    HRESULT STDMETHODCALLTYPE Deactivate() override { return real_->Deactivate(); }
    HRESULT STDMETHODCALLTYPE CreateDocumentMgr(ITfDocumentMgr** doc) override { return real_->CreateDocumentMgr(doc); }
    HRESULT STDMETHODCALLTYPE EnumDocumentMgrs(IEnumTfDocumentMgrs** docs) override { return real_->EnumDocumentMgrs(docs); }
    HRESULT STDMETHODCALLTYPE GetFocus(ITfDocumentMgr** doc) override { return real_->GetFocus(doc); }
    HRESULT STDMETHODCALLTYPE SetFocus(ITfDocumentMgr* doc) override { return real_->SetFocus(doc); }
    HRESULT STDMETHODCALLTYPE AssociateFocus(HWND window,ITfDocumentMgr* doc,ITfDocumentMgr** previous) override { return real_->AssociateFocus(window,doc,previous); }
    HRESULT STDMETHODCALLTYPE IsThreadFocus(BOOL* focused) override { return real_->IsThreadFocus(focused); }
    HRESULT STDMETHODCALLTYPE GetFunctionProvider(REFCLSID id,ITfFunctionProvider** provider) override { return real_->GetFunctionProvider(id,provider); }
    HRESULT STDMETHODCALLTYPE EnumFunctionProviders(IEnumTfFunctionProviders** providers) override { return real_->EnumFunctionProviders(providers); }
    HRESULT STDMETHODCALLTYPE GetGlobalCompartment(ITfCompartmentMgr** mgr) override { return real_->GetGlobalCompartment(mgr); }
    HRESULT STDMETHODCALLTYPE AdviseKeyEventSink(TfClientId,ITfKeyEventSink* sink,BOOL) override { if(sink_) return E_UNEXPECTED; sink_=sink; return S_OK; }
    HRESULT STDMETHODCALLTYPE UnadviseKeyEventSink(TfClientId) override { sink_.Reset(); return S_OK; }
    HRESULT STDMETHODCALLTYPE GetForeground(CLSID*) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE TestKeyDown(WPARAM,LPARAM,BOOL*) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE TestKeyUp(WPARAM,LPARAM,BOOL*) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE KeyDown(WPARAM,LPARAM,BOOL*) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE KeyUp(WPARAM,LPARAM,BOOL*) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetPreservedKey(ITfContext*,const TF_PRESERVEDKEY*,GUID*) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE IsPreservedKey(REFGUID,const TF_PRESERVEDKEY*,BOOL*) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE PreserveKey(TfClientId,REFGUID,const TF_PRESERVEDKEY*,const WCHAR*,ULONG) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE UnpreserveKey(REFGUID,const TF_PRESERVEDKEY*) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE SetPreservedKeyDescription(REFGUID,const WCHAR*,ULONG) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetPreservedKeyDescription(REFGUID,BSTR*) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE SimulatePreservedKey(ITfContext*,REFGUID,BOOL*) override { return E_NOTIMPL; }
};
