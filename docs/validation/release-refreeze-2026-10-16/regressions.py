"""Run the committed release regressions against an owned HTTPS production fixture.

Only harness imports/origin/SQL fixture identity/output paths are adapted. The
assertions and production code remain the committed RELEASE-RECOVERY versions.
"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]


def run(r):
    if not __debug__:
        raise RuntimeError('Optimized Python disables required release validation')
    root = r.root/'release-regressions'; root.mkdir(mode=0o700)
    original = REPO/'frontend/e2e/releaseRecovery.browser.ts'
    script = original.read_text()
    replacements = {
        "from '@playwright/test'": "from "+json.dumps(str(REPO/'frontend/node_modules/@playwright/test/index.mjs')),
        "from '../src/": "from '"+str(REPO/'frontend/src')+'/',
        "from './": "from '"+str(REPO/'frontend/e2e')+'/',
        "'golf-release-recovery-db'": json.dumps(r.state['services']['postgres']['id']),
        "'-U', 'golf', '-d', 'golf'": "'-U', "+json.dumps(r.state['values']['POSTGRES_OWNER_USER'])+", '-d', "+json.dumps(r.state['values']['POSTGRES_DB']),
        '/tmp/golf-release-recovery-': str(root/'shot-'),
    }
    for before, after in replacements.items():
        assert before in script, 'Expected committed test harness shape changed'
        script = script.replace(before, after)
    # Existing helper explicitly creates another context with a dev baseURL.
    # Preserve every assertion while binding that context to this HTTPS origin.
    needle = 'claimManager(f, browser, 1)'
    assert script.count(needle) == 1
    proxy = '''claimManager(f, new Proxy(browser, { get(target, key, receiver) {
      if (key === 'newContext') return (options: Parameters<typeof browser.newContext>[0]) => target.newContext({ ...options, baseURL: ORIGIN, ignoreHTTPSErrors: true })
      return Reflect.get(target, key, receiver)
    } }), 1)'''.replace('ORIGIN', json.dumps(r.state['origin']))
    script = script.replace(needle, proxy)
    test = root/'releaseRecovery.browser.ts'; test.write_text(script)
    config = root/'playwright.config.ts'
    config.write_text('import { defineConfig } from '+json.dumps(str(REPO/'frontend/node_modules/@playwright/test/index.mjs'))+'\nexport default defineConfig('+json.dumps({
        'testDir': str(root), 'testMatch': '*.browser.ts', 'workers': 1, 'timeout': 240000,
        'outputDir': str(root/'results'), 'use': {'baseURL': r.state['origin'],
        'browserName': 'chromium', 'channel': 'chrome', 'headless': True,
        'ignoreHTTPSErrors': True, 'actionTimeout': 15000, 'screenshot': 'only-on-failure', 'trace': 'off'}
    })+')\n')
    env = r.env.copy(); env['GOLF_RELEASE_RECOVERY_BROWSER'] = '1'
    result = subprocess.run([str(REPO/'frontend/node_modules/.bin/playwright'), 'test', '--config', str(config), '--reporter=json'],
        cwd=REPO/'frontend', env=env, capture_output=True, text=True, timeout=750)
    (root/'report.json').write_text(result.stdout); (root/'stderr.log').write_text(result.stderr)
    assert result.returncode == 0, 'Production regression failed; private diagnostic retained'
    report = json.loads(result.stdout)
    assert report['stats']['expected'] == 3 and report['stats']['unexpected'] == report['stats']['skipped'] == 0
    images = {name: data for name, data in r.state['images'].items()}
    checks = {'real_admin_demotion_initial403_abandon_no_extra_write': True,
        'committed_original_denied_exact_replay_abandon_keeps_history': True,
        'four_ball_confirm_pending_guard_resolve_reconfirm_lock_and_hidden_scorer': True}
    (HERE/'release-regressions.json').write_text(json.dumps({'checks': checks, 'images': images,
        'source_test': str(original.relative_to(REPO)), 'source_test_sha256': hashlib.sha256(original.read_bytes()).hexdigest(),
        'adapted_test_sha256': hashlib.sha256(script.encode()).hexdigest(), 'assertions_unchanged': True,
        'harness_adaptations': ['absolute imports', 'HTTPS origin for both contexts', 'owned SQL container/role/database', 'private screenshot paths'],
        'passed': 3, 'failed': 0, 'widths': [320, 390, 1280],
        'browser': 'Playwright chromium, channel chrome',
        'elapsed_seconds': round(report['stats']['duration']/1000, 3)}, indent=2)+'\n')
    for name in ('denied-initial-320', 'denied-replay-1280', 'hidden-final-scorer-320', 'guarded-lock-320', 'guarded-lock-1280', 'resolved-locked-390'):
        shutil.copyfile(root/('shot-'+name+'.png'), HERE/('regression-'+name+'.png'))
    for name in checks:
        r.receipt(name)
