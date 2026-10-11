// SPDX-License-Identifier: AGPL-3.0-only
#include "common.h"
static HRESULT module_path(std::wstring& path) {
    HMODULE module=nullptr;
    if(!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        reinterpret_cast<LPCWSTR>(&module_path),&module)) return HRESULT_FROM_WIN32(GetLastError());
    wchar_t buffer[32768]; DWORD n=GetModuleFileNameW(module,buffer,32768);
    if(!n || n>=32768) return E_FAIL;
    path.assign(buffer,n); return S_OK;
}
static HRESULT owned_path(const std::wstring& expected) {
    wchar_t buffer[32768]; DWORD size=sizeof(buffer);
    std::wstring key=std::wstring(CLASS_KEY)+L"\\InprocServer32";
    LSTATUS status=RegGetValueW(HKEY_CLASSES_ROOT,key.c_str(),nullptr,RRF_RT_REG_SZ,nullptr,buffer,&size);
    if(status==ERROR_FILE_NOT_FOUND) return S_FALSE;
    if(status!=ERROR_SUCCESS) return HRESULT_FROM_WIN32(status);
    return _wcsicmp(buffer,expected.c_str())==0 ? S_OK : HRESULT_FROM_WIN32(ERROR_ALREADY_EXISTS);
}
static HRESULT write_value(const std::wstring& key,const wchar_t* name,const std::wstring& value) {
    HKEY handle=nullptr;
    LSTATUS status=RegCreateKeyExW(HKEY_CLASSES_ROOT,key.c_str(),0,nullptr,0,KEY_WRITE,nullptr,&handle,nullptr);
    if(status!=ERROR_SUCCESS) return HRESULT_FROM_WIN32(status);
    status=RegSetValueExW(handle,name,0,REG_SZ,reinterpret_cast<const BYTE*>(value.c_str()),static_cast<DWORD>((value.size()+1)*sizeof(wchar_t)));
    RegCloseKey(handle); return HRESULT_FROM_WIN32(status);
}
static HRESULT unregister_service(const std::wstring& path) {
    HRESULT hr=owned_path(path); if(hr!=S_OK) return hr==S_FALSE ? S_OK : hr;
    ComPtr<ITfCategoryMgr> category;
    hr=CoCreateInstance(CLSID_TF_CategoryMgr,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&category)); if(FAILED(hr)) return hr;
    hr=category->UnregisterCategory(CLSID_NextIME,GUID_TFCAT_TIP_KEYBOARD,CLSID_NextIME); if(FAILED(hr)) return hr;
    ComPtr<ITfInputProcessorProfiles> profiles;
    hr=CoCreateInstance(CLSID_TF_InputProcessorProfiles,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&profiles)); if(FAILED(hr)) return hr;
    hr=profiles->Unregister(CLSID_NextIME); if(FAILED(hr)) return hr;
    LSTATUS status=RegDeleteTreeW(HKEY_CLASSES_ROOT,CLASS_KEY);
    return status==ERROR_FILE_NOT_FOUND ? S_OK : HRESULT_FROM_WIN32(status);
}
extern "C" __declspec(dllexport) HRESULT WINAPI DllRegisterServer() {
    HRESULT init=CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED);
    if(FAILED(init) && init!=RPC_E_CHANGED_MODE) return init;
    HRESULT hr=E_FAIL;
    try {
        std::wstring path; hr=module_path(path);
        if(SUCCEEDED(hr)) hr=owned_path(path);
        // Refuse to replace any existing installation, including the same path.
        if(hr==S_OK) hr=HRESULT_FROM_WIN32(ERROR_ALREADY_EXISTS);
        else if(hr==S_FALSE) {
            hr=write_value(CLASS_KEY,nullptr,L"Luqnexa Japanese (prototype)");
            if(SUCCEEDED(hr)) hr=write_value(std::wstring(CLASS_KEY)+L"\\InprocServer32",nullptr,path);
            if(SUCCEEDED(hr)) hr=write_value(std::wstring(CLASS_KEY)+L"\\InprocServer32",L"ThreadingModel",L"Apartment");
            ComPtr<ITfInputProcessorProfiles> profiles; ComPtr<ITfCategoryMgr> category;
            if(SUCCEEDED(hr)) hr=CoCreateInstance(CLSID_TF_InputProcessorProfiles,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&profiles));
            if(SUCCEEDED(hr)) hr=profiles->Register(CLSID_NextIME);
            const wchar_t description[]=L"Luqnexa Japanese (prototype)";
            if(SUCCEEDED(hr)) hr=profiles->AddLanguageProfile(CLSID_NextIME,JAPANESE,PROFILE_NextIME,description,
                static_cast<ULONG>(wcslen(description)),path.c_str(),static_cast<ULONG>(path.size()),0);
            if(SUCCEEDED(hr)) hr=CoCreateInstance(CLSID_TF_CategoryMgr,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&category));
            if(SUCCEEDED(hr)) hr=category->RegisterCategory(CLSID_NextIME,GUID_TFCAT_TIP_KEYBOARD,CLSID_NextIME);
            if(FAILED(hr)) {
                if(category) category->UnregisterCategory(CLSID_NextIME,GUID_TFCAT_TIP_KEYBOARD,CLSID_NextIME);
                if(profiles) profiles->Unregister(CLSID_NextIME);
                RegDeleteTreeW(HKEY_CLASSES_ROOT,CLASS_KEY);
            }
        }
    } catch(...) { hr=E_FAIL; }
    if(SUCCEEDED(init)) CoUninitialize(); return hr;
}
extern "C" __declspec(dllexport) HRESULT WINAPI DllUnregisterServer() {
    HRESULT init=CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED);
    if(FAILED(init) && init!=RPC_E_CHANGED_MODE) return init;
    HRESULT hr;
    try { std::wstring path; hr=module_path(path); if(SUCCEEDED(hr)) hr=unregister_service(path); } catch(...) { hr=E_FAIL; }
    if(SUCCEEDED(init)) CoUninitialize(); return hr;
}
