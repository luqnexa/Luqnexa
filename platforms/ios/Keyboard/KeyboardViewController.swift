// SPDX-License-Identifier: AGPL-3.0-only
import UIKit

final class KeyboardViewController: UIInputViewController {
    private var handle: UInt64 = 0
    private let composition = UILabel()
    private let choices = UIStackView()
    private var shifted = false
    private var letters: [UIButton] = []

    override func viewDidLoad() {
        super.viewDidLoad()
        let code = nextime_session_create(nil, 0, 8, &handle)
        let layout = UIStackView()
        layout.axis = .vertical; layout.spacing = 5
        layout.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(layout)
        NSLayoutConstraint.activate([
            layout.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 6),
            layout.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -6),
            layout.topAnchor.constraint(equalTo: view.topAnchor, constant: 5),
            layout.bottomAnchor.constraint(equalTo: view.bottomAnchor, constant: -5),
            view.heightAnchor.constraint(equalToConstant: 290)
        ])
        composition.text = code == 0 ? "日本語 / ローマ字入力" : "辞書を開けません"
        composition.font = .systemFont(ofSize: 16)
        layout.addArrangedSubview(composition)
        let scroll = UIScrollView(); scroll.heightAnchor.constraint(equalToConstant: 38).isActive = true
        choices.axis = .horizontal; choices.spacing = 8
        choices.translatesAutoresizingMaskIntoConstraints = false
        scroll.addSubview(choices)
        NSLayoutConstraint.activate([
            choices.leadingAnchor.constraint(equalTo: scroll.contentLayoutGuide.leadingAnchor),
            choices.trailingAnchor.constraint(equalTo: scroll.contentLayoutGuide.trailingAnchor),
            choices.topAnchor.constraint(equalTo: scroll.contentLayoutGuide.topAnchor),
            choices.bottomAnchor.constraint(equalTo: scroll.contentLayoutGuide.bottomAnchor),
            choices.heightAnchor.constraint(equalTo: scroll.frameLayoutGuide.heightAnchor)
        ])
        layout.addArrangedSubview(scroll)
        for row in ["1234567890", "qwertyuiop", "asdfghjkl", "zxcvbnm"] {
            let stack = UIStackView(); stack.spacing = 3; stack.distribution = .fillEqually
            for ch in row {
                let title = String(ch)
                let key = button(title) { [weak self] in
                    guard let self = self else { return }
                    self.event(1, self.shifted ? title.uppercased() : title)
                }
                if ch.isLetter { letters.append(key) }
                stack.addArrangedSubview(key)
            }
            layout.addArrangedSubview(stack)
        }
        let controls = UIStackView(); controls.spacing = 3; controls.distribution = .fillEqually
        controls.addArrangedSubview(button("🌐") { [weak self] in self?.advanceToNextInputMode() })
        controls.addArrangedSubview(button("⇧") { [weak self] in
            guard let self = self else { return }
            self.shifted.toggle()
            for key in self.letters {
                let title = key.title(for: .normal) ?? ""
                key.setTitle(self.shifted ? title.uppercased() : title.lowercased(), for: .normal)
            }
        })
        controls.addArrangedSubview(button("日/英") { [weak self] in self?.event(9) })
        controls.addArrangedSubview(button("全角") { [weak self] in self?.event(16) })
        controls.addArrangedSubview(button("半角") { [weak self] in self?.event(17) })
        controls.addArrangedSubview(button("⌫") { [weak self] in
            guard let self = self else { return }
            if self.text(1).isEmpty { self.textDocumentProxy.deleteBackward() } else { self.event(2) }
        })
        layout.addArrangedSubview(controls)
        let actions = UIStackView(); actions.spacing = 3; actions.distribution = .fillEqually
        actions.addArrangedSubview(button("取消") { [weak self] in self?.event(7) })
        actions.addArrangedSubview(button("空白") { [weak self] in
            guard let self = self else { return }
            if self.text(1).isEmpty { self.textDocumentProxy.insertText(" ") } else { self.event(10) }
        })
        actions.addArrangedSubview(button("変換") { [weak self] in self?.event(3) })
        actions.addArrangedSubview(button("確定") { [weak self] in self?.event(6) })
        actions.addArrangedSubview(button("。") { [weak self] in self?.event(1, "。") })
        layout.addArrangedSubview(actions)
        // No history, network, timers, or model training in the extension.
        event(14) // Disable the existing session's default text learning.
    }
    deinit { if handle != 0 { _ = nextime_session_destroy(handle) } }

    private func button(_ title: String, action: @escaping () -> Void) -> UIButton {
        let key = UIButton(type: .system)
        key.setTitle(title, for: .normal)
        key.backgroundColor = .secondarySystemBackground
        key.layer.cornerRadius = 5
        key.titleLabel?.font = .systemFont(ofSize: 15)
        key.addAction(UIAction { _ in action() }, for: .touchUpInside)
        return key
    }
    private func text(_ kind: UInt32, _ index: UInt32 = 0) -> String {
        var required = 0
        guard nextime_session_text(handle, kind, index, nil, 0, &required) == 4, required <= 8192 else { return "" }
        var bytes = [UInt8](repeating: 0, count: required)
        let code = bytes.withUnsafeMutableBufferPointer {
            nextime_session_text(handle, kind, index, $0.baseAddress, $0.count, &required)
        }
        guard code == 0 else { return "" }
        return String(decoding: bytes.dropLast(), as: UTF8.self)
    }
    private func event(_ key: UInt32, _ value: String = "") {
        guard handle != 0 else { return }
        var flags: UInt32 = 0
        let code: Int32
        if value.isEmpty { code = nextime_session_event(handle, key, nil, 0, &flags) }
        else {
            let bytes = Array(value.utf8)
            code = bytes.withUnsafeBufferPointer {
                nextime_session_event(handle, key, $0.baseAddress, $0.count, &flags)
            }
        }
        guard code == 0 else { composition.text = "変換エラー（\(code)）"; return }
        if flags & 2 != 0 { textDocumentProxy.insertText(text(5)) }
        composition.text = text(3).isEmpty ? text(6) + " / ローマ字入力" : text(3)
        for view in choices.arrangedSubviews { choices.removeArrangedSubview(view); view.removeFromSuperview() }
        var count: UInt32 = 0; var selected: UInt32 = 0
        guard nextime_session_candidates(handle, &count, &selected) == 0 else { return }
        for index in 0..<count {
            choices.addArrangedSubview(button(text(4, index)) { [weak self] in
                guard let self = self else { return }
                var n: UInt32 = 0; var current: UInt32 = 0
                guard nextime_session_candidates(self.handle, &n, &current) == 0, n > 0 else { return }
                if current == UInt32.max { self.event(4); current = 0 }
                for _ in 0..<((index + n - current) % n) { self.event(4) }
                // Explicit candidate tap commits; never send a message.
                self.event(6)
            })
        }
    }
}
