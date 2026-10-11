// SPDX-License-Identifier: AGPL-3.0-only
#pragma once
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <msctf.h>
#include <textstor.h>
#include <inputscope.h>
#include <wrl/client.h>
#include <atomic>
#include <string>
#include <vector>
#include <stdexcept>
#include <cstring>
#include "nextime.h"
using Microsoft::WRL::ComPtr;
inline constexpr CLSID CLSID_NextIME = {0x97d8a216,0x4d8c,0x4ad5,{0x98,0x39,0xaf,0x53,0xea,0x9a,0xdb,0x41}};
inline constexpr GUID PROFILE_NextIME = {0x329c2a21,0x6ec1,0x40f1,{0xad,0x28,0x20,0xdc,0xf4,0x97,0x14,0xef}};
inline constexpr wchar_t CLASS_KEY[] = L"CLSID\\{97D8A216-4D8C-4AD5-9839-AF53EA9ADB41}";
inline constexpr LANGID JAPANESE = 0x0411;
inline std::wstring widen(const std::string& text) {
    if(text.empty()) return {};
    int count=MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,text.data(),static_cast<int>(text.size()),nullptr,0);
    if(!count) throw std::runtime_error("invalid UTF-8");
    std::wstring result(static_cast<size_t>(count),L'\0');
    MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,text.data(),static_cast<int>(text.size()),result.data(),count);
    return result;
}
struct KeyAction { uint32_t key=0; std::string text; };
inline KeyAction map_key(UINT vk, bool ctrl, bool alt, bool win, bool shift, bool composing, bool candidates) {
    if(ctrl || alt || win) return {};
    if(vk==VK_F12 || vk==VK_KANJI) return {NEXTIME_TOGGLE_MODE,{}};
    if(vk==VK_F10 && composing) return {NEXTIME_LITERAL,{}};
    if(vk==VK_F9) return {NEXTIME_TOGGLE_AI,{}};
    if(vk==VK_F8) return {NEXTIME_CLEAR_LEARNING,{}};
    if(vk==VK_F7) return {NEXTIME_TOGGLE_TRAINING,{}};
    if(vk>='A' && vk<='Z') return {NEXTIME_TEXT,std::string(1,static_cast<char>(shift ? vk : 'a'+vk-'A'))};
    if(vk>='0' && vk<='9') return {NEXTIME_TEXT,std::string(1,static_cast<char>(vk))};
    if(vk>=VK_NUMPAD0 && vk<=VK_NUMPAD9) return {NEXTIME_TEXT,std::string(1,static_cast<char>('0'+vk-VK_NUMPAD0))};
    if(vk>=VK_OEM_1 && vk<=VK_OEM_8) return {NEXTIME_TEXT,"?"};
    if(vk==VK_OEM_102) return {NEXTIME_TEXT,"?"};
    if(vk==VK_MULTIPLY) return {NEXTIME_TEXT,"*"};
    if(vk==VK_ADD) return {NEXTIME_TEXT,"+"};
    if(vk==VK_SUBTRACT) return {NEXTIME_TEXT,"-"};
    if(vk==VK_DIVIDE) return {NEXTIME_TEXT,"/"};
    if(vk==VK_DECIMAL) return {NEXTIME_TEXT,"."};
    if(vk==VK_OEM_MINUS && !shift) return {NEXTIME_TEXT,"-"};
    if(vk==VK_OEM_COMMA && !shift) return {NEXTIME_TEXT,","};
    if(vk==VK_OEM_PERIOD && !shift) return {NEXTIME_TEXT,"."};
    if(vk==VK_OEM_7 && !shift) return {NEXTIME_TEXT,"'"};
    if(!composing) return {};
    if(vk==VK_SPACE) return {static_cast<uint32_t>(shift && candidates ? NEXTIME_PREVIOUS : NEXTIME_SPACE),{}};
    if(vk==VK_BACK) return {NEXTIME_BACKSPACE,{}};
    if(vk==VK_RETURN) return {NEXTIME_ENTER,{}};
    if(vk==VK_ESCAPE) return {NEXTIME_CANCEL,{}};
    if(candidates && (vk==VK_DOWN || vk==VK_UP)) return {static_cast<uint32_t>(vk==VK_DOWN ? NEXTIME_NEXT : NEXTIME_PREVIOUS),{}};
    return {};
}

class RustSession {
    HMODULE module_=nullptr;
    uint64_t handle_=0;
    decltype(&nextime_session_create) create_=nullptr;
    decltype(&nextime_session_destroy) destroy_=nullptr;
    decltype(&nextime_session_event) event_=nullptr;
    decltype(&nextime_session_text) text_=nullptr;
    decltype(&nextime_session_candidates) candidates_=nullptr;
    template<class T> T symbol(const char* name) {
        auto value=GetProcAddress(module_,name);
        if(!value) throw std::runtime_error("missing Rust ABI export");
        T result; static_assert(sizeof(result)==sizeof(value));
        std::memcpy(&result,&value,sizeof(result)); return result;
    }
public:
    RustSession()=default;
    RustSession(const RustSession&)=delete;
    ~RustSession() { if(handle_) destroy_(handle_); if(module_) FreeLibrary(module_); }
    void initialize(HMODULE adapter) {
        if(handle_) return;
        if(module_) { FreeLibrary(module_); module_=nullptr; }
        wchar_t path[32768]; DWORD len=GetModuleFileNameW(adapter,path,32768);
        if(!len || len>=32768) throw std::runtime_error("adapter path");
        std::wstring directory(path,len); directory.resize(directory.find_last_of(L"\\/")+1);
        module_=LoadLibraryExW((directory+L"nextime_ffi.dll").c_str(),nullptr,LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR|LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
        if(!module_) throw std::runtime_error("Rust DLL load");
        auto version=symbol<decltype(&nextime_abi_version)>("nextime_abi_version");
        if(version()!=1) throw std::runtime_error("Rust ABI version");
        create_=symbol<decltype(create_)>("nextime_session_create");
        destroy_=symbol<decltype(destroy_)>("nextime_session_destroy");
        event_=symbol<decltype(event_)>("nextime_session_event");
        text_=symbol<decltype(text_)>("nextime_session_text");
        candidates_=symbol<decltype(candidates_)>("nextime_session_candidates");
        // Prefer a local NUD (index checked on open, blocks checked on demand).
        std::wstring nud=directory+L"base.nud";
        std::string utf8;
        if(GetFileAttributesW(nud.c_str())!=INVALID_FILE_ATTRIBUTES) {
            int bytes=WideCharToMultiByte(CP_UTF8,0,nud.data(),static_cast<int>(nud.size()),nullptr,0,nullptr,nullptr);
            utf8.resize(static_cast<size_t>(bytes));
            WideCharToMultiByte(CP_UTF8,0,nud.data(),static_cast<int>(nud.size()),utf8.data(),bytes,nullptr,nullptr);
        }
        if(create_(reinterpret_cast<const uint8_t*>(utf8.data()),utf8.size(),24,&handle_)!=NEXTIME_OK) throw std::runtime_error("Rust session create");
    }
    uint32_t event(const KeyAction& action) {
        uint32_t flags=0;
        if(event_(handle_,action.key,reinterpret_cast<const uint8_t*>(action.text.data()),action.text.size(),&flags)!=NEXTIME_OK) throw std::runtime_error("Rust event");
        return flags;
    }
    void clear_context() { if(handle_) event({NEXTIME_CLEAR_CONTEXT,{}}); }
    std::wstring text(uint32_t kind,uint32_t index=0) const {
        size_t size=0;
        if(text_(handle_,kind,index,nullptr,0,&size)!=NEXTIME_BUFFER_TOO_SMALL) throw std::runtime_error("Rust size query");
        std::vector<uint8_t> bytes(size);
        if(text_(handle_,kind,index,bytes.data(),bytes.size(),&size)!=NEXTIME_OK) throw std::runtime_error("Rust text");
        return widen(std::string(reinterpret_cast<char*>(bytes.data()),size-1));
    }
    uint32_t candidates(uint32_t* selected=nullptr) const {
        uint32_t count=0,index=0;
        if(candidates_(handle_,&count,&index)!=NEXTIME_OK) throw std::runtime_error("Rust candidates");
        if(selected) *selected=index;
        return count;
    }
};
