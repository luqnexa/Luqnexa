// SPDX-License-Identifier: AGPL-3.0-only
#include "common.h"
#include "candidates.h"
#include <new>
static HMODULE module_handle=nullptr;
static std::atomic<long> live_objects{0},server_locks{0};
class Service;

class Edit final : public ITfEditSession {
    std::atomic<ULONG> references_{1};
    Service* service_;
    ComPtr<ITfContext> context_;
    KeyAction action_;
public:
    bool handled=false;
    Edit(Service* service,ITfContext* context,KeyAction action);
    ~Edit();
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** out) override {
        if(!out) return E_POINTER; *out=nullptr;
        if(iid==IID_IUnknown || iid==IID_ITfEditSession) { *out=static_cast<ITfEditSession*>(this); AddRef(); return S_OK; } return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++references_; }
    ULONG STDMETHODCALLTYPE Release() override { ULONG n=--references_; if(!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie cookie) override;
};

class Service final : public ITfTextInputProcessor, public ITfKeyEventSink, public ITfCompositionSink, public ITfThreadMgrEventSink {
    std::atomic<ULONG> references_{1};
    ComPtr<ITfThreadMgr> manager_;
    ComPtr<ITfComposition> composition_;
    ComPtr<ITfContext> context_;
    TfClientId client_=TF_CLIENTID_NULL;
    DWORD focus_cookie_=TF_INVALID_COOKIE;
    RustSession rust_;
    CandidateWindow window_;
    bool key_sink_=false;
    bool composing_=false,candidates_=false;
    bool cancel_pending_=false;
    HRESULT request(ITfContext* context,KeyAction action,BOOL* eaten=nullptr,DWORD mode=TF_ES_SYNC) {
        if(eaten) *eaten=FALSE;
        if(!context) return E_INVALIDARG;
        ComPtr<ITfEditSession> edit; auto implementation=new(std::nothrow) Edit(this,context,std::move(action));
        if(!implementation) return E_OUTOFMEMORY; edit.Attach(implementation);
        HRESULT session_result=E_FAIL;
        HRESULT hr=context->RequestEditSession(client_,edit.Get(),mode|TF_ES_READWRITE,&session_result);
        if(eaten) *eaten=implementation->handled;
        return FAILED(hr) ? hr : session_result;
    }
    KeyAction action(WPARAM key) const {
        if(cancel_pending_) return {};
        KeyAction result=map_key(static_cast<UINT>(key),(GetKeyState(VK_CONTROL)&0x8000)!=0,(GetKeyState(VK_MENU)&0x8000)!=0,
            ((GetKeyState(VK_LWIN)|GetKeyState(VK_RWIN))&0x8000)!=0,(GetKeyState(VK_SHIFT)&0x8000)!=0,composing_,candidates_);
        if(result.key==NEXTIME_TEXT) {
            // Translate printable keys using the user's actual JIS/US layout and modifiers.
            BYTE state[256]{}; WCHAR text[8]{}; HKL layout=GetKeyboardLayout(0);
            if(!GetKeyboardState(state)) return {};
            if(key==VK_OEM_3 && LOWORD(reinterpret_cast<ULONG_PTR>(layout))==JAPANESE) return {NEXTIME_TOGGLE_MODE,{}};
            int count=ToUnicodeEx(static_cast<UINT>(key),MapVirtualKeyExW(static_cast<UINT>(key),MAPVK_VK_TO_VSC,layout),state,text,8,4,layout);
            if(count<=0) return {}; // Dead keys are passed through, without changing their state.
            int bytes=WideCharToMultiByte(CP_UTF8,0,text,count,nullptr,0,nullptr,nullptr);
            result.text.resize(static_cast<size_t>(bytes)); WideCharToMultiByte(CP_UTF8,0,text,count,result.text.data(),bytes,nullptr,nullptr);
        }
        return result;
    }
    void reset() {
        rust_.event({NEXTIME_CANCEL,{}}); composing_=false; candidates_=false; cancel_pending_=false; window_.hide();
    }
    void cancel_on_focus_loss() {
        rust_.clear_context();
        window_.hide();
        if(context_ && composition_ && !cancel_pending_) {
            cancel_pending_=true;
            try { if(FAILED(request(context_.Get(),{NEXTIME_CANCEL,{}},nullptr,TF_ES_ASYNCDONTCARE))) cancel_pending_=false; }
            catch(...) { cancel_pending_=false; }
        }
    }
public:
    Service() { ++live_objects; }
    ~Service() { --live_objects; }
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** out) override {
        if(!out) return E_POINTER; *out=nullptr;
        if(iid==IID_IUnknown || iid==IID_ITfTextInputProcessor) *out=static_cast<ITfTextInputProcessor*>(this);
        else if(iid==IID_ITfKeyEventSink) *out=static_cast<ITfKeyEventSink*>(this);
        else if(iid==IID_ITfCompositionSink) *out=static_cast<ITfCompositionSink*>(this);
        else if(iid==IID_ITfThreadMgrEventSink) *out=static_cast<ITfThreadMgrEventSink*>(this);
        else return E_NOINTERFACE;
        AddRef(); return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++references_; }
    ULONG STDMETHODCALLTYPE Release() override { ULONG n=--references_; if(!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE Activate(ITfThreadMgr* manager,TfClientId client) override {
        try {
            if(!manager || manager_) return E_INVALIDARG;
            rust_.initialize(module_handle); manager_=manager; client_=client;
            ComPtr<ITfSource> source; HRESULT hr=manager_.As(&source);
            if(SUCCEEDED(hr)) hr=source->AdviseSink(IID_ITfThreadMgrEventSink,static_cast<ITfThreadMgrEventSink*>(this),&focus_cookie_);
            ComPtr<ITfKeystrokeMgr> keys;
            if(SUCCEEDED(hr)) hr=manager_.As(&keys);
            if(SUCCEEDED(hr)) hr=keys->AdviseKeyEventSink(client_,static_cast<ITfKeyEventSink*>(this),TRUE);
            if(FAILED(hr)) { Deactivate(); return hr; }
            key_sink_=true;
            return S_OK;
        } catch(...) { Deactivate(); return E_FAIL; }
    }
    HRESULT STDMETHODCALLTYPE Deactivate() override {
        try {
            cancel_on_focus_loss();
            if(manager_) {
                ComPtr<ITfKeystrokeMgr> keys; if(key_sink_ && SUCCEEDED(manager_.As(&keys))) keys->UnadviseKeyEventSink(client_);
                ComPtr<ITfSource> source; if(focus_cookie_!=TF_INVALID_COOKIE && SUCCEEDED(manager_.As(&source))) source->UnadviseSink(focus_cookie_);
            }
            key_sink_=false; focus_cookie_=TF_INVALID_COOKIE; manager_.Reset();
            return S_OK;
        } catch(...) { return E_FAIL; }
    }
    HRESULT STDMETHODCALLTYPE OnSetFocus(BOOL foreground) override { if(!foreground) cancel_on_focus_loss(); return S_OK; }
    HRESULT STDMETHODCALLTYPE OnTestKeyDown(ITfContext* context,WPARAM key,LPARAM,BOOL* eaten) override {
        if(!eaten) return E_POINTER; *eaten=FALSE;
        TF_STATUS status{};
        if(!context || FAILED(context->GetStatus(&status)) || (status.dwDynamicFlags&TS_SD_READONLY)) return S_OK;
        try { *eaten=action(key).key!=0; return S_OK; } catch(...) { return E_FAIL; }
    }
    HRESULT STDMETHODCALLTYPE OnKeyDown(ITfContext* context,WPARAM key,LPARAM,BOOL* eaten) override {
        if(!eaten) return E_POINTER; *eaten=FALSE;
        try {
            KeyAction input=action(key); if(!input.key || !context) return S_OK;
            // Mutate Rust state only inside a granted synchronous document lock.
            request(context,std::move(input),eaten); return S_OK;
        } catch(...) { window_.hide(); return E_FAIL; }
    }
    HRESULT STDMETHODCALLTYPE OnTestKeyUp(ITfContext*,WPARAM,LPARAM,BOOL* eaten) override { if(!eaten) return E_POINTER; *eaten=FALSE; return S_OK; }
    HRESULT STDMETHODCALLTYPE OnKeyUp(ITfContext*,WPARAM,LPARAM,BOOL* eaten) override { if(!eaten) return E_POINTER; *eaten=FALSE; return S_OK; }
    HRESULT STDMETHODCALLTYPE OnPreservedKey(ITfContext*,REFGUID,BOOL* eaten) override { if(!eaten) return E_POINTER; *eaten=FALSE; return S_OK; }
    HRESULT STDMETHODCALLTYPE OnCompositionTerminated(TfEditCookie,ITfComposition* composition) override {
        try {
            if(composition_.Get()==composition) { composition_.Reset(); context_.Reset(); reset(); }
            return S_OK;
        } catch(...) { return E_FAIL; }
    }
    HRESULT STDMETHODCALLTYPE OnInitDocumentMgr(ITfDocumentMgr*) override { return S_OK; }
    HRESULT STDMETHODCALLTYPE OnUninitDocumentMgr(ITfDocumentMgr*) override { return S_OK; }
    HRESULT STDMETHODCALLTYPE OnPushContext(ITfContext*) override { return S_OK; }
    HRESULT STDMETHODCALLTYPE OnPopContext(ITfContext* context) override { if(context_.Get()==context) cancel_on_focus_loss(); return S_OK; }
    HRESULT STDMETHODCALLTYPE OnSetFocus(ITfDocumentMgr*,ITfDocumentMgr*) override { cancel_on_focus_loss(); return S_OK; }

    HRESULT edit(ITfContext* context,TfEditCookie cookie,const KeyAction& action,bool& handled) {
        if(context_ && context_.Get()!=context) return S_FALSE;
        if(action.key!=NEXTIME_CANCEL) {
            TF_STATUS status{}; HRESULT hr=context->GetStatus(&status); if(FAILED(hr)) return hr;
            if(status.dwDynamicFlags&TS_SD_READONLY) return S_FALSE;
            // Honor TSF's disabled/empty-context compartments.
            ComPtr<ITfCompartmentMgr> compartments;
            if(SUCCEEDED(context->QueryInterface(IID_PPV_ARGS(&compartments)))) {
                for(const GUID* id : {&GUID_COMPARTMENT_KEYBOARD_DISABLED,&GUID_COMPARTMENT_EMPTYCONTEXT}) {
                    ComPtr<ITfCompartment> compartment;
                    if(SUCCEEDED(compartments->GetCompartment(*id,&compartment))) {
                        VARIANT value; VariantInit(&value); HRESULT value_hr=compartment->GetValue(&value);
                        bool blocked=SUCCEEDED(value_hr) && value.vt==VT_I4 && value.lVal!=0; VariantClear(&value);
                        if(blocked) return S_FALSE;
                    }
                }
            }
            TF_SELECTION selection{}; ULONG fetched=0;
            if(SUCCEEDED(context->GetSelection(cookie,TF_DEFAULT_SELECTION,1,&selection,&fetched)) && fetched) {
                ComPtr<ITfRange> range; range.Attach(selection.range);
                ComPtr<ITfReadOnlyProperty> property;
                if(SUCCEEDED(context->GetAppProperty(GUID_PROP_INPUTSCOPE,&property))) {
                    VARIANT value; VariantInit(&value);
                    if(SUCCEEDED(property->GetValue(cookie,range.Get(),&value)) && value.vt==VT_UNKNOWN && value.punkVal) {
                        ComPtr<ITfInputScope> scope;
                        if(SUCCEEDED(value.punkVal->QueryInterface(IID_PPV_ARGS(&scope)))) {
                            InputScope* scopes=nullptr; UINT count=0;
                            if(SUCCEEDED(scope->GetInputScopes(&scopes,&count))) {
                                bool password=false; for(UINT i=0;i<count;++i) if(scopes[i]==IS_PASSWORD || scopes[i]==IS_NUMERIC_PASSWORD) password=true;
                                CoTaskMemFree(scopes); if(password) { VariantClear(&value); return S_FALSE; }
                            }
                        }
                    }
                    VariantClear(&value);
                }
            }
        }
        if(!composition_ && action.key==NEXTIME_TEXT) {
            ComPtr<ITfInsertAtSelection> insert; HRESULT hr=context->QueryInterface(IID_PPV_ARGS(&insert)); if(FAILED(hr)) return hr;
            ComPtr<ITfRange> range; hr=insert->InsertTextAtSelection(cookie,TF_IAS_QUERYONLY,L"",0,&range); if(FAILED(hr)) return hr;
            ComPtr<ITfContextComposition> compositions; hr=context->QueryInterface(IID_PPV_ARGS(&compositions)); if(FAILED(hr)) return hr;
            hr=compositions->StartComposition(cookie,range.Get(),this,&composition_); if(FAILED(hr) || !composition_) return FAILED(hr) ? hr : S_FALSE;
            context_=context;
        }
        uint32_t flags=rust_.event(action); handled=(flags&NEXTIME_HANDLED)!=0;
        if(action.key==NEXTIME_POLL_AI && !handled) return S_OK;
        if(action.key==NEXTIME_CANCEL) cancel_pending_=false;
        composing_=!rust_.text(NEXTIME_RAW).empty(); candidates_=rust_.candidates()!=0;
        if(!composition_) return S_OK;
        ComPtr<ITfRange> range; HRESULT hr=composition_->GetRange(&range); if(FAILED(hr)) return hr;
        std::wstring text=(flags&NEXTIME_HAS_COMMIT) ? rust_.text(NEXTIME_COMMITTED) : rust_.text(NEXTIME_DISPLAY);
        hr=range->SetText(cookie,0,text.data(),static_cast<LONG>(text.size())); if(FAILED(hr)) return hr;
        ComPtr<ITfRange> cursor; hr=range->Clone(&cursor); if(FAILED(hr)) return hr;
        hr=cursor->Collapse(cookie,TF_ANCHOR_END); if(FAILED(hr)) return hr;
        TF_SELECTION selection{cursor.Get(),{TF_AE_NONE,FALSE}}; hr=context->SetSelection(cookie,1,&selection); if(FAILED(hr)) return hr;
        if(!composing_) {
            auto composition=composition_; composition_.Reset(); context_.Reset();
            hr=composition->EndComposition(cookie); window_.hide(); return hr;
        }
        if(candidates_) {
            ComPtr<ITfContextView> view; RECT caret{}; BOOL clipped=FALSE;
            if(SUCCEEDED(context->GetActiveView(&view)) && SUCCEEDED(view->GetTextExt(cookie,cursor.Get(),&caret,&clipped))) window_.show(module_handle,rust_,caret);
            else window_.hide();
        } else window_.hide();
        return S_OK;
    }
};

Edit::Edit(Service* service,ITfContext* context,KeyAction action):service_(service),context_(context),action_(std::move(action)) { ++live_objects; service_->AddRef(); }
Edit::~Edit() { service_->Release(); --live_objects; }
HRESULT Edit::DoEditSession(TfEditCookie cookie) { try { return service_->edit(context_.Get(),cookie,action_,handled); } catch(...) { return E_FAIL; } }

class Factory final : public IClassFactory {
    std::atomic<ULONG> references_{1};
public:
    Factory() { ++live_objects; } ~Factory() { --live_objects; }
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** out) override { if(!out) return E_POINTER; *out=nullptr; if(iid!=IID_IUnknown && iid!=IID_IClassFactory) return E_NOINTERFACE; *out=this; AddRef(); return S_OK; }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++references_; }
    ULONG STDMETHODCALLTYPE Release() override { ULONG n=--references_; if(!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE CreateInstance(IUnknown* outer,REFIID iid,void** out) override {
        if(!out) return E_POINTER; *out=nullptr; if(outer) return CLASS_E_NOAGGREGATION;
        try { auto service=new Service; HRESULT hr=service->QueryInterface(iid,out); service->Release(); return hr; } catch(...) { return E_OUTOFMEMORY; }
    }
    HRESULT STDMETHODCALLTYPE LockServer(BOOL lock) override { if(lock) ++server_locks; else if(server_locks>0) --server_locks; return S_OK; }
};
HRESULT WINAPI DllGetClassObject(REFCLSID clsid,REFIID iid,void** out) {
    if(!out) return E_POINTER; *out=nullptr; if(clsid!=CLSID_NextIME) return CLASS_E_CLASSNOTAVAILABLE;
    auto factory=new(std::nothrow) Factory; if(!factory) return E_OUTOFMEMORY;
    HRESULT hr=factory->QueryInterface(iid,out); factory->Release(); return hr;
}
HRESULT WINAPI DllCanUnloadNow() { return live_objects==0 && server_locks==0 ? S_OK : S_FALSE; }
BOOL WINAPI DllMain(HINSTANCE instance,DWORD reason,void*) { if(reason==DLL_PROCESS_ATTACH) { module_handle=instance; DisableThreadLibraryCalls(instance); } return TRUE; }
