# SPDX-License-Identifier: AGPL-3.0-only
import hashlib
import importlib.util
import json
from pathlib import Path
import plistlib
import struct
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('verify', Path(__file__).with_name('verify-ios-probe.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ArchiveTests(unittest.TestCase):
    def archive(self, path, keyboard=True, open_access=False, architecture=0x0100000C):
        base = 'Payload/LuqnexaProbe.app/'
        model = b'LQML0001' + bytes(165684)
        with zipfile.ZipFile(path, 'w') as archive:
            archive.writestr(base + 'Info.plist', plistlib.dumps(dict(CFBundleIdentifier='org.luqnexa.probe', CFBundleExecutable='App', MinimumOSVersion='16.0')))
            archive.writestr(base + 'App', struct.pack('<II', 0xFEEDFACF, 0x0100000C))
            archive.writestr(base + 'model.lqml', model)
            archive.writestr(base + 'training-report.json', json.dumps(dict(model_sha256=hashlib.sha256(model).hexdigest())))
            if keyboard:
                ext = base + 'PlugIns/LuqnexaKeyboard.appex/'
                archive.writestr(ext + 'Info.plist', plistlib.dumps(dict(CFBundleIdentifier='org.luqnexa.probe.keyboard', CFBundleExecutable='Keyboard', NSExtension=dict(NSExtensionPointIdentifier='com.apple.keyboard-service', NSExtensionAttributes=dict(RequestsOpenAccess=open_access)))))
                archive.writestr(ext + 'Keyboard', struct.pack('<II', 0xFEEDFACF, architecture))

    def test_extension_required_and_privacy_architecture_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'fixture.ipa'
            self.archive(path)
            self.assertTrue(module.verify(path, True)['keyboard_extension'])
            for args in [dict(keyboard=False), dict(open_access=True), dict(architecture=0x01000007)]:
                self.archive(path, **args)
                with self.assertRaises(ValueError):
                    module.verify(path, True)


if __name__ == '__main__':
    unittest.main()
