// SPDX-License-Identifier: AGPL-3.0-only
#pragma once
#include "common.h"
#include <algorithm>
class CandidateWindow {
    HWND window_=nullptr;
    HMODULE module_=nullptr;
    std::wstring class_name_;
    std::vector<std::wstring> rows_;
    uint32_t selected_=0;
    int row_height_=28;
    static LRESULT CALLBACK procedure(HWND hwnd,UINT msg,WPARAM wparam,LPARAM lparam) {
        auto self=reinterpret_cast<CandidateWindow*>(GetWindowLongPtrW(hwnd,GWLP_USERDATA));
        if(msg==WM_NCCREATE) {
            self=static_cast<CandidateWindow*>(reinterpret_cast<CREATESTRUCTW*>(lparam)->lpCreateParams);
            SetWindowLongPtrW(hwnd,GWLP_USERDATA,reinterpret_cast<LONG_PTR>(self));
        }
        if(msg==WM_MOUSEACTIVATE) return MA_NOACTIVATE;
        if(msg==WM_ERASEBKGND) return 1;
        if(msg==WM_PAINT && self) {
            PAINTSTRUCT paint{}; HDC dc=BeginPaint(hwnd,&paint);
            auto font=static_cast<HFONT>(GetStockObject(DEFAULT_GUI_FONT)); auto old=SelectObject(dc,font);
            RECT area{}; GetClientRect(hwnd,&area); FillRect(dc,&area,GetSysColorBrush(COLOR_WINDOW));
            for(size_t i=0;i<self->rows_.size();++i) {
                RECT row{0,static_cast<LONG>(i)*self->row_height_,area.right,(static_cast<LONG>(i)+1)*self->row_height_};
                const bool selected=i==self->selected_;
                FillRect(dc,&row,GetSysColorBrush(selected ? COLOR_HIGHLIGHT : COLOR_WINDOW));
                SetTextColor(dc,GetSysColor(selected ? COLOR_HIGHLIGHTTEXT : COLOR_WINDOWTEXT));
                SetBkMode(dc,TRANSPARENT); row.left=8;
                DrawTextW(dc,self->rows_[i].c_str(),-1,&row,DT_SINGLELINE|DT_VCENTER|DT_NOPREFIX);
            }
            SelectObject(dc,old); EndPaint(hwnd,&paint); return 0;
        }
        return DefWindowProcW(hwnd,msg,wparam,lparam);
    }
public:
    ~CandidateWindow() { if(window_) DestroyWindow(window_); if(module_) UnregisterClassW(class_name_.c_str(),module_); }
    void hide() { if(window_) ShowWindow(window_,SW_HIDE); }
    HRESULT show(HMODULE module,const RustSession& session,const RECT& caret) {
        uint32_t selected=0,count=session.candidates(&selected);
        if(!count) { hide(); return S_OK; }
        rows_.clear();
        uint32_t start=selected==UINT32_MAX ? 0 : (selected/8)*8;
        rows_.push_back(session.text(NEXTIME_MODE)+L" / "+session.text(NEXTIME_AI_MODE)+L" F9 / F8:消去 / F10:英字 / F12:日英");
        rows_.push_back(session.text(NEXTIME_TRAINING_MODE)+L" F7:文章学習の切替（端末内）");
        auto local=session.text(NEXTIME_LOCAL_AI_MODE);
        rows_.push_back(local==L"local:pending" ? L"文章理解AI:端末内 / 処理中" :
            local==L"local:on" ? L"文章理解AI:端末内 / 有効" : L"文章理解AI:無効");
        for(uint32_t i=start;i<std::min(count,start+8);++i) rows_.push_back(std::to_wstring(i+1)+L". "+session.text(NEXTIME_CANDIDATE,i));
        selected_=selected==UINT32_MAX ? UINT32_MAX : selected-start+3;
        if(!window_) {
            WNDCLASSW klass{}; klass.lpfnWndProc=procedure; klass.hInstance=module;
            class_name_=L"Luqnexa.Candidates."+std::to_wstring(reinterpret_cast<uintptr_t>(this));
            klass.lpszClassName=class_name_.c_str(); klass.hCursor=LoadCursorW(nullptr,MAKEINTRESOURCEW(32512));
            if(!RegisterClassW(&klass)) return HRESULT_FROM_WIN32(GetLastError());
            module_=module;
            window_=CreateWindowExW(WS_EX_NOACTIVATE|WS_EX_TOOLWINDOW|WS_EX_TOPMOST,klass.lpszClassName,L"Luqnexa",WS_POPUP|WS_BORDER,0,0,320,240,nullptr,nullptr,module,this);
            if(!window_) return HRESULT_FROM_WIN32(GetLastError());
        }
        UINT dpi=GetDpiForWindow(window_); row_height_=MulDiv(28,static_cast<int>(dpi ? dpi : 96),96);
        int width=MulDiv(360,static_cast<int>(dpi ? dpi : 96),96),height=row_height_*static_cast<int>(rows_.size())+2;
        MONITORINFO monitor{sizeof(monitor)}; GetMonitorInfoW(MonitorFromRect(&caret,MONITOR_DEFAULTTONEAREST),&monitor);
        int x=std::clamp(static_cast<int>(caret.left),static_cast<int>(monitor.rcWork.left),std::max(static_cast<int>(monitor.rcWork.left),static_cast<int>(monitor.rcWork.right)-width));
        int y=caret.bottom; if(y+height>monitor.rcWork.bottom) y=std::max(static_cast<int>(monitor.rcWork.top),static_cast<int>(caret.top)-height);
        SetWindowPos(window_,HWND_TOPMOST,x,y,width,height,SWP_NOACTIVATE|SWP_SHOWWINDOW);
        InvalidateRect(window_,nullptr,FALSE); return S_OK;
    }
};
