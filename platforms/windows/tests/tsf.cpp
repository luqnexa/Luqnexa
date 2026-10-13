// SPDX-License-Identifier: AGPL-3.0-only
#include "common.h"
#include "manager.h"
#include <algorithm>
#include <iostream>
#include <chrono>
static void check(bool success,const char* message) { if(!success) throw std::runtime_error(message); }
static void ok(HRESULT hr,const char* message) { if(FAILED(hr)) { std::cerr<<message<<" HRESULT="<<std::hex<<hr<<std::dec<<"\n"; throw std::runtime_error(message); } }
// Minimal, in-memory ACP host. No registry registration or external app input.
class Store final : public ITextStoreACP {
    ULONG refs_=1; ComPtr<ITextStoreACPSink> sink_; DWORD lock_=0;
    TS_SELECTION_ACP selection_{0,0,{TS_AE_END,FALSE}};
public:
    std::wstring value; bool readonly=false,reject_lock=false;
    void set_readonly(bool enabled) { readonly=enabled; if(sink_) ok(sink_->OnStatusChange(TS_SD_READONLY),"status change"); }
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** out) override { if(!out) return E_POINTER; *out=nullptr; if(iid!=IID_IUnknown && iid!=IID_ITextStoreACP) return E_NOINTERFACE; *out=this; AddRef(); return S_OK; }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++refs_; }
    ULONG STDMETHODCALLTYPE Release() override { ULONG n=--refs_; if(!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE AdviseSink(REFIID iid,IUnknown* sink,DWORD) override { if(iid!=IID_ITextStoreACPSink) return E_NOINTERFACE; return sink->QueryInterface(IID_PPV_ARGS(&sink_)); }
    HRESULT STDMETHODCALLTYPE UnadviseSink(IUnknown*) override { sink_.Reset(); return S_OK; }
    HRESULT STDMETHODCALLTYPE RequestLock(DWORD flags,HRESULT* result) override { if(!result) return E_POINTER; if(lock_ || reject_lock) { *result=TS_E_SYNCHRONOUS; return S_OK; } if(!sink_) return E_FAIL; lock_=flags; *result=sink_->OnLockGranted(flags); lock_=0; return S_OK; }
    HRESULT STDMETHODCALLTYPE GetStatus(TS_STATUS* status) override { *status={static_cast<DWORD>(readonly ? TS_SD_READONLY : 0),0}; return S_OK; }
    HRESULT STDMETHODCALLTYPE QueryInsert(LONG start,LONG end,ULONG,LONG* first,LONG* last) override { if(start<0 || end<start || static_cast<size_t>(end)>value.size()) return TS_E_INVALIDPOS; *first=start; *last=end; return S_OK; }
    HRESULT STDMETHODCALLTYPE GetSelection(ULONG index,ULONG count,TS_SELECTION_ACP* selection,ULONG* fetched) override { *fetched=0; if(!lock_) return TS_E_NOLOCK; if(count && (index==TS_DEFAULT_SELECTION || index==0)) { *selection=selection_; *fetched=1; } return S_OK; }
    HRESULT STDMETHODCALLTYPE SetSelection(ULONG count,const TS_SELECTION_ACP* selection) override { if(!lock_) return TS_E_NOLOCK; if(count!=1) return E_INVALIDARG; selection_=*selection; return S_OK; }
    HRESULT STDMETHODCALLTYPE GetText(LONG start,LONG end,WCHAR* plain,ULONG capacity,ULONG* copied,TS_RUNINFO* runs,ULONG run_capacity,ULONG* run_count,LONG* next) override {
        if(!lock_) return TS_E_NOLOCK; if(end==-1) end=static_cast<LONG>(value.size());
        if(start<0 || end<start || static_cast<size_t>(end)>value.size()) return TS_E_INVALIDPOS;
        *copied=std::min(capacity,static_cast<ULONG>(end-start));
        if(*copied) std::copy_n(value.data()+start,*copied,plain);
        *run_count=run_capacity && end>start ? 1 : 0;
        if(*run_count) runs[0]={capacity ? *copied : static_cast<ULONG>(end-start),TS_RT_PLAIN};
        *next=start+static_cast<LONG>(capacity ? *copied : end-start); return S_OK;
    }
    HRESULT STDMETHODCALLTYPE SetText(DWORD,LONG start,LONG end,const WCHAR* text,ULONG count,TS_TEXTCHANGE* change) override {
        if((lock_&TS_LF_READWRITE)!=TS_LF_READWRITE) return TS_E_NOLOCK;
        if(readonly) return TS_E_READONLY;
        if(start<0 || end<start || static_cast<size_t>(end)>value.size()) return TS_E_INVALIDPOS;
        value.replace(static_cast<size_t>(start),static_cast<size_t>(end-start),text,count);
        *change={start,end,start+static_cast<LONG>(count)}; selection_={change->acpNewEnd,change->acpNewEnd,{TS_AE_END,FALSE}}; return S_OK;
    }
    HRESULT STDMETHODCALLTYPE InsertTextAtSelection(DWORD flags,const WCHAR* text,ULONG count,LONG* start,LONG* end,TS_TEXTCHANGE* change) override {
        if(!lock_) return TS_E_NOLOCK; LONG first=selection_.acpStart,last=selection_.acpEnd;
        if(start) *start=first; if(end) *end=last;
        if(flags&TS_IAS_QUERYONLY) return S_OK;
        HRESULT hr=SetText(0,first,last,text,count,change); if(end && SUCCEEDED(hr)) *end=change->acpNewEnd; return hr;
    }
    HRESULT STDMETHODCALLTYPE GetFormattedText(LONG,LONG,IDataObject**) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetEmbedded(LONG,REFGUID,REFIID,IUnknown**) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE QueryInsertEmbedded(const GUID*,const FORMATETC*,BOOL* possible) override { *possible=FALSE; return S_OK; }
    HRESULT STDMETHODCALLTYPE InsertEmbedded(DWORD,LONG,LONG,IDataObject*,TS_TEXTCHANGE*) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE InsertEmbeddedAtSelection(DWORD,IDataObject*,LONG*,LONG*,TS_TEXTCHANGE*) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE RequestSupportedAttrs(DWORD,ULONG,const TS_ATTRID*) override { return S_OK; }
    HRESULT STDMETHODCALLTYPE RequestAttrsAtPosition(LONG,ULONG,const TS_ATTRID*,DWORD) override { return S_OK; }
    HRESULT STDMETHODCALLTYPE RequestAttrsTransitioningAtPosition(LONG,ULONG,const TS_ATTRID*,DWORD) override { return S_OK; }
    HRESULT STDMETHODCALLTYPE FindNextAttrTransition(LONG,LONG halt,ULONG,const TS_ATTRID*,DWORD,LONG* next,BOOL* found,LONG* offset) override { *next=halt; *found=FALSE; *offset=0; return S_OK; }
    HRESULT STDMETHODCALLTYPE RetrieveRequestedAttrs(ULONG,TS_ATTRVAL*,ULONG* fetched) override { *fetched=0; return S_OK; }
    HRESULT STDMETHODCALLTYPE GetEndACP(LONG* end) override { *end=static_cast<LONG>(value.size()); return S_OK; }
    HRESULT STDMETHODCALLTYPE GetActiveView(TsViewCookie* view) override { *view=0; return S_OK; }
    HRESULT STDMETHODCALLTYPE GetACPFromPoint(TsViewCookie,const POINT*,DWORD,LONG*) override { return TS_E_NOLAYOUT; }
    HRESULT STDMETHODCALLTYPE GetTextExt(TsViewCookie,LONG,LONG,RECT*,BOOL*) override { return TS_E_NOLAYOUT; }
    HRESULT STDMETHODCALLTYPE GetScreenExt(TsViewCookie,RECT* rect) override { *rect={0,0,640,480}; return S_OK; }
    HRESULT STDMETHODCALLTYPE GetWnd(TsViewCookie,HWND* window) override { *window=nullptr; return S_OK; }
};
template<class T> static T symbol(HMODULE module,const char* name) { FARPROC address=GetProcAddress(module,name); check(address!=nullptr,name); T result; static_assert(sizeof(result)==sizeof(address)); std::memcpy(&result,&address,sizeof(result)); return result; }
static void registered_activation() {
    ComPtr<ITfThreadMgr> manager;
    ok(CoCreateInstance(CLSID_TF_ThreadMgr,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&manager)),"registered manager");
    TfClientId owner=0; ok(manager->Activate(&owner),"registered manager activate");
    ComPtr<ITfInputProcessorProfileMgr> profiles;
    ok(CoCreateInstance(CLSID_TF_InputProcessorProfiles,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&profiles)),"profile manager");
    HRESULT activation=profiles->ActivateProfile(TF_PROFILETYPE_INPUTPROCESSOR,JAPANESE,CLSID_NextIME,PROFILE_NextIME,nullptr,TF_IPPMF_FORPROCESS);
    ok(activation,"process profile activation"); check(activation==S_OK,"profile enabled for process");
    ComPtr<ITfKeystrokeMgr> keys; ok(manager.As(&keys),"real dispatcher");
    CLSID foreground{}; ok(keys->GetForeground(&foreground),"foreground service"); check(foreground==CLSID_NextIME,"NextIME foreground service");
    ok(profiles->DeactivateProfile(TF_PROFILETYPE_INPUTPROCESSOR,JAPANESE,CLSID_NextIME,PROFILE_NextIME,nullptr,TF_IPPMF_FORPROCESS),"process profile deactivate");
    keys.Reset(); profiles.Reset(); ok(manager->Deactivate(),"registered manager deactivate"); manager.Reset();
    CoFreeUnusedLibraries();
    std::cout<<"Registered activation passed: OS activated NextIME and installed its foreground key sink; process profile deactivation passed. Keyboard delivery/rendering in foreground apps remain unverified.\n";
}
int wmain(int argc,wchar_t** argv) {
    if(argc!=2 && argc!=3) return 2;
    const bool registered=argc==3 && wcscmp(argv[2],L"--registered")==0;
    if(argc==3 && !registered) return 2;
    HRESULT init=CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED); if(FAILED(init)) return 2;
    HMODULE module=nullptr; int result=0;
    try {
        module=LoadLibraryExW(argv[1],nullptr,LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR|LOAD_LIBRARY_SEARCH_DEFAULT_DIRS); check(module!=nullptr,"Load DLL");
        auto factory_fn=symbol<HRESULT(WINAPI*)(REFCLSID,REFIID,void**)>(module,"DllGetClassObject");
        auto can_unload=symbol<HRESULT(WINAPI*)()>(module,"DllCanUnloadNow");
        symbol<HRESULT(WINAPI*)()>(module,"DllRegisterServer"); symbol<HRESULT(WINAPI*)()>(module,"DllUnregisterServer");
        check(can_unload()==S_OK,"initial DLL lifetime");
        if(registered) {
            ComPtr<ITfInputProcessorProfiles> profiles;
            ok(CoCreateInstance(CLSID_TF_InputProcessorProfiles,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&profiles)),"profiles");
            ComPtr<IEnumTfLanguageProfiles> languages; ok(profiles->EnumLanguageProfiles(JAPANESE,&languages),"enumerate Japanese profiles");
            TF_LANGUAGEPROFILE profile{}; ULONG fetched=0; bool found=false;
            while(languages->Next(1,&profile,&fetched)==S_OK && fetched) {
                if(profile.clsid==CLSID_NextIME && profile.guidProfile==PROFILE_NextIME) found=true;
            }
            check(found,"NextIME registered profile");
            BSTR description=nullptr; ok(profiles->GetLanguageProfileDescription(CLSID_NextIME,JAPANESE,PROFILE_NextIME,&description),"profile description");
            check(description!=nullptr,"profile description present"); SysFreeString(description);
            BOOL enabled=FALSE; ok(profiles->IsEnabledLanguageProfile(CLSID_NextIME,JAPANESE,PROFILE_NextIME,&enabled),"profile enabled state");
            std::cout<<"Registered Japanese profile found; enabled="<<(enabled ? "true" : "false")<<"\n";
            registered_activation();
        }
        {
            ComPtr<IClassFactory> factory; ok(factory_fn(CLSID_NextIME,IID_PPV_ARGS(&factory)),"factory");
            check(can_unload()==S_FALSE,"factory lifetime");
            void* invalid=nullptr; check(factory_fn(GUID_NULL,IID_IUnknown,&invalid)==CLASS_E_CLASSNOTAVAILABLE,"unknown class");
            ComPtr<ITfTextInputProcessor> service; ok(factory->CreateInstance(nullptr,IID_PPV_ARGS(&service)),"service");
            ComPtr<ITfThreadMgr> manager; ok(CoCreateInstance(CLSID_TF_ThreadMgr,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&manager)),"thread manager");
            TfClientId client=0; ok(manager->Activate(&client),"manager activate");
            TfClientId owner=client;
            ComPtr<ITfClientId> ids; ok(manager.As(&ids),"client IDs");
            ok(ids->GetClientId(CLSID_NextIME,&client),"service client ID");
            ComPtr<TestManager> proxy; proxy.Attach(new TestManager(manager.Get()));
            ITfThreadMgr* service_manager=static_cast<ITfThreadMgr*>(proxy.Get());
            ok(service->Activate(service_manager,client),"service activate");
            ComPtr<ITfKeyEventSink> keys; ok(service.As(&keys),"key sink");
            ComPtr<Store> store; store.Attach(new Store);
            ComPtr<ITfDocumentMgr> document; ok(manager->CreateDocumentMgr(&document),"document");
            ComPtr<ITfContext> context; TfEditCookie cookie=0;
            ok(document->CreateContext(owner,0,store.Get(),&context,&cookie),"context"); ok(document->Push(context.Get()),"push");
            ok(manager->SetFocus(document.Get()),"focus");
            auto key=[&](UINT vk,bool expected=true) { BOOL eaten=FALSE; ok(keys->OnKeyDown(context.Get(),vk,0,&eaten),"key event"); check((eaten!=FALSE)==expected,"key handled"); };
            key(VK_RETURN,false);
            key(VK_F9); key(VK_F9);
            key(VK_F7); key(VK_F7);
            store->set_readonly(true); key('A',false); check(store->value.empty(),"readonly"); store->set_readonly(false);
            store->reject_lock=true; key('H',false); check(store->value.empty(),"lock refusal"); store->reject_lock=false;
            for(UINT vk : {'H','A','S','H','I'}) key(vk);
            check(store->value==L"はし","preedit"); key(VK_SPACE); check(store->value==L"橋","first candidate");
            key(VK_DOWN); check(store->value==L"箸","next candidate"); key(VK_RETURN); check(store->value==L"箸","explicit commit");
            for(UINT vk : {'H','A','S','H','I'}) key(vk);
            key(VK_SPACE); check(store->value==L"箸箸","learned candidate");
            key(VK_ESCAPE); key(VK_F8);
            key('A'); check(store->value==L"箸あ","next composition"); key(VK_ESCAPE); check(store->value==L"箸","cancel");
            key('N'); check(store->value==L"箸n","pending n"); key(VK_BACK); check(store->value==L"箸","backspace");
            for(UINT vk : {'O','M','G'}) key(vk);
            check(store->value==L"箸omg","slang preedit"); key(VK_RETURN); check(store->value==L"箸omg","slang commit");
            key('K'); key('A'); key(VK_F10); check(store->value==L"箸omgka","literal Latin"); key(VK_RETURN);
            key(VK_F12); key('H'); key('I'); key('2'); key(VK_SPACE);
            check(store->value==L"箸omgkahi2 ","English space commit"); key(VK_F12);
            const std::wstring mixed_prefix=store->value;
            for(UINT vk : {'S','E','X'}) key(vk);
            check(store->value==mixed_prefix+L"sex","sex Latin preedit");
            for(UINT vk : {'G','A'}) key(vk);
            check(store->value==mixed_prefix+L"sexが","mixed Latin kana preedit");
            key(VK_SPACE); check(store->value==mixed_prefix+L"sexが","mixed conversion");
            key(VK_RETURN); check(store->value==mixed_prefix+L"sexが","mixed commit");
            check(!map_key('A',true,false,false,false,false,false).key,"Ctrl shortcut");
            check(!map_key('A',false,true,false,false,false,false).key,"Alt shortcut");
            check(!map_key('A',false,false,true,false,false,false).key,"Windows shortcut");
            check(map_key(VK_SPACE,false,false,false,true,true,true).key==NEXTIME_PREVIOUS,"Shift Space");
            check(map_key(VK_F6,false,false,false,false,true,false).key==NEXTIME_FULL_WIDTH,"full width shortcut");
            check(map_key(VK_F6,false,false,false,true,true,false).key==NEXTIME_HALF_WIDTH,"half width shortcut");
            check(map_key(VK_F10,false,false,false,true,true,false).key==NEXTIME_FULL_WIDTH,"full width Latin shortcut");
            for(UINT vk='A';vk<='Z';++vk) {
                check(map_key(vk,false,false,false,true,false,false).text==std::string(1,static_cast<char>(vk)),"Shift uppercase mapping");
                check(map_key(vk,false,false,false,false,false,false).text==std::string(1,static_cast<char>('a'+vk-'A')),"plain lowercase mapping");
            }
            {
                RustSession shifted; shifted.initialize(module);
                for(UINT vk : {'A','I'}) shifted.event(map_key(vk,false,false,false,true,false,false));
                check(shifted.text(2)==L"AI","Shift AI preedit");
                shifted.event({NEXTIME_ENTER,{}});
                check(shifted.text(NEXTIME_COMMITTED)==L"AI","Shift AI commit");
                for(UINT vk : {'X','Y','Z'}) shifted.event(map_key(vk,false,false,false,true,false,false));
                check(shifted.text(2)==L"XYZ","unknown Shift letters");
                shifted.event({NEXTIME_SPACE,{}});
                check(shifted.text(NEXTIME_DISPLAY)==L"XYZ","uppercase conversion preserves text");
                shifted.event({NEXTIME_ENTER,{}});
                check(shifted.text(NEXTIME_COMMITTED)==L"XYZ","unknown uppercase commit");
            }
            std::vector<double> timings;
            for(int i=0;i<1100;++i) {
                // Same initial personal/context state for each timed sample.
                // The learning and clearing behavior is tested above.
                key(VK_F8);
                auto start=std::chrono::steady_clock::now();
                for(UINT vk : {'H','A','S','H','I'}) key(vk);
                key(VK_SPACE); key(VK_DOWN); key(VK_RETURN);
                auto end=std::chrono::steady_clock::now();
                check(store->value.back()==L'箸',"benchmark commit");
                if(i>=100) timings.push_back(std::chrono::duration<double,std::milli>(end-start).count());
            }
            std::sort(timings.begin(),timings.end());
            std::cout<<"TSF host benchmark: dictionary=adjacent-NUD-or-bundled iterations=1000 warmup=100 events=8 p50_ms="<<timings[499]<<" p95_ms="<<timings[949]<<" max_ms="<<timings.back()<<" (no rendering/registration)\n";
            const std::wstring baseline=store->value;
            key('A'); ok(keys->OnSetFocus(FALSE),"focus loss");
            auto deadline=std::chrono::steady_clock::now()+std::chrono::seconds(2);
            while(store->value!=baseline && std::chrono::steady_clock::now()<deadline) {
                MSG message{}; while(PeekMessageW(&message,nullptr,0,0,PM_REMOVE)) { TranslateMessage(&message); DispatchMessageW(&message); }
                MsgWaitForMultipleObjects(0,nullptr,FALSE,10,QS_ALLINPUT);
            }
            check(store->value==baseline,"focus cancellation");
            // Exercise disable/enable lifecycle on the same service object.
            ok(service->Deactivate(),"service deactivate"); ok(service->Activate(service_manager,client),"reactivate");
            ok(service->Deactivate(),"deactivate again");
            ok(manager->SetFocus(nullptr),"clear focus"); ok(document->Pop(TF_POPF_ALL),"pop");
            context.Reset(); document.Reset(); store.Reset(); keys.Reset(); service.Reset(); factory.Reset();
            proxy.Reset(); ids.Reset(); ok(manager->Deactivate(),"manager deactivate"); manager.Reset();
        }
        check(can_unload()==S_OK,"final DLL lifetime");
        std::cout<<"TSF integration passed: COM lifecycle, preedit, candidates, navigation, commit, cancel, backspace, shortcuts, readonly, lock refusal, focus cancellation, reactivation.\n";
    } catch(const std::exception& error) { std::cerr<<error.what()<<"\n"; result=1; }
    // On test failure process exit cleans up: do not unload a still-live COM DLL.
    if(module && !result) FreeLibrary(module);
    CoUninitialize(); return result;
}
