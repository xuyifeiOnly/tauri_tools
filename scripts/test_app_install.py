import pathlib
import plistlib
import tempfile
import unittest
from unittest.mock import patch
import subprocess

from app_bundles import candidates, cleanup, validate_target, unregister


IDENTIFIER = 'cn.rutang.mac-tunnel'


class AppBundlesTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = pathlib.Path(self.tmp.name)

    def app(self, relative, identifier=IDENTIFIER):
        app = self.root / relative
        (app / 'Contents').mkdir(parents=True)
        (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier': identifier}))
        return app

    def test_cleanup_preserves_target_and_is_idempotent(self):
        target = self.app('Applications/随享.app')
        old = self.app('Applications/Mac Tunnel.app')
        build = self.app('build/Mac Tunnel.app')
        other = self.app('Applications/Other.app', 'other.app')
        roots = [target.parent, build.parent]
        calls = []
        cleanup(target, roots, calls.append)
        cleanup(target, roots, calls.append)
        self.assertTrue(target.exists())
        self.assertTrue(other.exists())
        self.assertFalse(old.exists())
        self.assertFalse(build.exists())
        self.assertEqual(candidates(roots), [target])

    def test_rejects_foreign_target(self):
        with self.assertRaises(ValueError):
            validate_target(self.app('随享.app', 'other.app'))

    def test_rejects_symlink_without_touching_destination(self):
        app = self.app('outside/Original.app')
        link = self.root / 'Applications/随享.app'
        link.parent.mkdir()
        link.symlink_to(app, target_is_directory=True)
        with self.assertRaises(ValueError):
            candidates([link.parent])
        self.assertTrue(app.exists())

    def test_does_not_scan_recursively(self):
        app = self.app('Applications/nested/Keep.app')
        self.assertEqual(candidates([self.root / 'Applications']), [])
        self.assertTrue(app.exists())

    def test_unregister_failure_does_not_remove_bundle(self):
        target = self.app('Applications/随享.app')
        old = self.app('Applications/Mac Tunnel.app')
        def fail(_):
            raise RuntimeError('registration failed')
        with self.assertRaises(RuntimeError):
            cleanup(target, [target.parent], fail)
        self.assertTrue(old.exists())
        self.assertTrue(target.exists())

    @patch('app_bundles.subprocess.run')
    def test_unregistered_build_is_registered_then_unregistered(self, run):
        run.side_effect = [subprocess.CompletedProcess([], 1, 'failed: -10814', ''),
                           subprocess.CompletedProcess([], 0), subprocess.CompletedProcess([], 0)]
        unregister(pathlib.Path('/build/随享.app'))
        self.assertEqual([call.args[0][1] for call in run.call_args_list], ['-u', '-f', '-u'])


if __name__ == '__main__':
    unittest.main()
