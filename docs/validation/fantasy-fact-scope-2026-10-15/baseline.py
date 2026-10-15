"""Build the pinned baseline and current candidate with the same debug profile."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
REF = 'e3ee9663c0e21178aa982d6682dbec7c23aa5de6'


def main():
    (HERE / 'builds.json').unlink(missing_ok=True)
    output = Path('/tmp/golf-a1u-baseline/golf-api')
    output.parent.mkdir(exist_ok=True)
    result = {'baseline_commit': REF, 'profile': 'debug'}
    with tempfile.TemporaryDirectory(prefix='golf-a1u-baseline-src-') as temp:
        archive = subprocess.check_output(['git', 'archive', REF], cwd=ROOT)
        subprocess.run(['tar', '-xf', '-', '-C', temp], input=archive, check=True)
        for phase, directory in [('baseline', Path(temp)), ('candidate', ROOT)]:
            print('START pinned build ' + phase, flush=True)
            with Path('/tmp/golf-a1u-' + phase + '-build.log').open('w') as log:
                if phase == 'candidate':
                    # Cargo's executable output path is shared by both workspaces;
                    # a cached candidate fingerprint cannot validate the baseline
                    # executable that just overwrote it. Force this package only.
                    subprocess.run(['cargo', 'clean', '-p', 'golf-api', '--target-dir', str(ROOT / 'target')],
                                   cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
                completed = subprocess.run(['cargo', 'build', '--locked', '-p', 'golf-api', '--bins',
                                            '--target-dir', str(ROOT / 'target')], cwd=directory,
                                           stdout=log, stderr=subprocess.STDOUT, timeout=1200)
            result[phase + '_build_exit'] = completed.returncode
            if completed.returncode:
                raise RuntimeError('Pinned build failed; inspect local log')
            binary = ROOT / 'target/debug/golf-api'
            with binary.open('rb') as stream:
                result[phase + '_binary_sha256'] = hashlib.file_digest(stream, 'sha256').hexdigest()
            if phase == 'baseline':
                shutil.copy2(binary, output)
    assert result['baseline_binary_sha256'] != result['candidate_binary_sha256']
    (HERE / 'builds.json').write_text(json.dumps(result, indent=2) + '\n')
    print('Pinned baseline and candidate built', flush=True)


if __name__ == '__main__':
    main()
