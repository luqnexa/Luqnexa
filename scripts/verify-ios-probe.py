# SPDX-License-Identifier: AGPL-3.0-only
"""Inspect a generated IPA without installing or executing it."""
import hashlib
import json
from pathlib import Path
import plistlib
import struct
import sys
import zipfile


def verify(path, require_keyboard=False):
    with zipfile.ZipFile(path) as archive:
        base = 'Payload/LuqnexaProbe.app/'
        names = archive.namelist()
        if any(n.startswith('/') or '..' in n.split('/') for n in names):
            raise ValueError('unsafe archive path')
        plist = plistlib.loads(archive.read(base + 'Info.plist'))
        if plist['CFBundleIdentifier'] != 'org.luqnexa.probe':
            raise ValueError('unexpected bundle identifier')
        executable = plist['CFBundleExecutable']
        if '/' in executable or executable in ('', '.', '..'):
            raise ValueError('invalid executable path')
        code = archive.read(base + executable)
        magic, cpu = struct.unpack_from('<II', code)
        if magic != 0xFEEDFACF or cpu != 0x0100000C:
            raise ValueError('expected a native arm64 Mach-O executable')
        model = archive.read(base + 'model.lqml')
        report = json.loads(archive.read(base + 'training-report.json'))
        digest = hashlib.sha256(model).hexdigest()
        if len(model) != 165692 or not model.startswith(b'LQML0001'):
            raise ValueError('invalid Rust model')
        if digest != report['model_sha256']:
            raise ValueError('embedded model SHA-256 mismatch')
        extension = base + 'PlugIns/LuqnexaKeyboard.appex/'
        keyboard = extension + 'Info.plist' in names
        if require_keyboard and not keyboard:
            raise ValueError('keyboard extension is missing')
        if keyboard:
            info = plistlib.loads(archive.read(extension + 'Info.plist'))
            metadata = info['NSExtension']
            if (info['CFBundleIdentifier'] != 'org.luqnexa.probe.keyboard'
                    or metadata['NSExtensionPointIdentifier'] != 'com.apple.keyboard-service'
                    or metadata['NSExtensionAttributes']['RequestsOpenAccess'] is not False):
                raise ValueError('unexpected keyboard identity or open access')
            binary = info['CFBundleExecutable']
            if '/' in binary or binary in ('', '.', '..'):
                raise ValueError('invalid keyboard executable path')
            if struct.unpack_from('<II', archive.read(extension + binary)) != (0xFEEDFACF, 0x0100000C):
                raise ValueError('keyboard must be native arm64')
        return dict(ipa_sha256=hashlib.sha256(Path(path).read_bytes()).hexdigest(),
                    model_sha256=digest, model_bytes=len(model),
                    executable_bytes=len(code), minimum_os=plist['MinimumOSVersion'],
                    architecture='arm64', keyboard_extension=keyboard,
                    installed=False, phone_executed=False,
                    scope='IPA structure and embedded model integrity, not runtime or performance')


if __name__ == '__main__':
    print(json.dumps(verify(sys.argv[1], '--require-keyboard' in sys.argv[2:]), indent=2))
