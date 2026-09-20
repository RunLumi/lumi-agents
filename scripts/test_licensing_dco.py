# SPDX-License-Identifier: Apache-2.0
import subprocess
import tempfile
import unittest
from pathlib import Path
from check_dco import identity, trailer_errors, check_range


class DCOTrailerTests(unittest.TestCase):
    def test_valid_author(self):
        self.assertEqual(trailer_errors('A Dev <a@invalid.example>', 'Signed-off-by: A Dev <a@invalid.example>'), [])

    def test_missing_signoff(self):
        self.assertTrue(trailer_errors('A Dev <a@invalid.example>', ''))

    def test_different_author(self):
        self.assertTrue(trailer_errors('A Dev <a@invalid.example>', 'Signed-off-by: Other <o@invalid.example>'))

    def test_coauthor_requires_own_signoff(self):
        body='Signed-off-by: A Dev <a@invalid.example>\nCo-authored-by: B Dev <b@invalid.example>'
        self.assertTrue(trailer_errors('A Dev <a@invalid.example>', body))
        self.assertEqual(trailer_errors('A Dev <a@invalid.example>', body+'\nSigned-off-by: B Dev <b@invalid.example>'), [])

    def test_case_and_noreply(self):
        self.assertEqual(trailer_errors('A Dev <123+dev@users.noreply.github.com>', 'signed-off-by: a dev <123+DEV@users.noreply.github.com>'), [])

    def test_malformed_and_forged_claim_not_accepted(self):
        self.assertIsNone(identity('A <no-address>'))
        self.assertTrue(trailer_errors('A <a@invalid.example>', 'Signed-off-by: CI approved automatically'))

    def test_git_range_and_policy_bootstrap(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d)
            def git(*args):
                return subprocess.check_output(['git','-C',d,*args], text=True).strip()
            git('init','-q');git('config','user.name','Synthetic Test Author');git('config','user.email','test@invalid.example')
            (root/'file').write_text('base');git('add','.');git('commit','-qm','fixture baseline');base=git('rev-parse','HEAD')
            (root/'DCO').write_text('test fixture');git('add','.');git('commit','-qm','policy fixture');policy=git('rev-parse','HEAD')
            self.assertEqual(check_range(root,base,policy),(False,[]))
            (root/'file').write_text('unsigned');git('add','.');git('commit','-qm','unsigned fixture');head=git('rev-parse','HEAD')
            self.assertTrue(check_range(root,policy,head)[1])
            # Synthetic test identity only; never a real contributor attestation.
            git('commit','--amend','-qm','signed fixture\n\nSigned-off-by: Synthetic Test Author <test@invalid.example>')
            self.assertEqual(check_range(root,policy,git('rev-parse','HEAD')),(True,[]))

    def test_injected_ref_rejected(self):
        with self.assertRaises(ValueError):
            check_range(Path('.'), '--all', '0'*40)


if __name__ == '__main__':
    unittest.main()
