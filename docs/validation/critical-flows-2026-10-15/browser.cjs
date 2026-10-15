const { fs, path, state, here, open } = require('./support.cjs');
const phase = process.argv[2];
async function main() {
  const f = await open();
  try {
    if (phase !== 'setup') await f.authenticate();
    await require(`./${phase}.cjs`)(f);
    await f.finish(phase);
  } catch (error) {
    fs.writeFileSync(path.join(state.directory, `private-${phase}-monitor.json`), JSON.stringify({ errors: f.errors, failures: f.failures, statuses: f.statuses }), { mode: 0o600 });
    for (const [index, context] of f.contexts.entries()) {
      const page = context.pages()[0];
      if (page) fs.writeFileSync(path.join(state.directory, `private-${phase}-${index}-live.json`), JSON.stringify(await page.evaluate(() => window.criticalLiveEvents || []).catch(() => [])), { mode: 0o600 });
      if (page) await page.screenshot({ path: path.join(state.directory, `private-${phase}-${index}.png`), fullPage: true }).catch(() => {});
    }
    throw error;
  } finally { await f.close(); }
}
main().catch(error => {
  fs.writeFileSync(path.join(state.directory, `critical-${phase}-failure.log`), String(error.stack), { mode: 0o600 });
  console.error(JSON.stringify({ phase, failed: error.name, detail: 'private diagnostics retained' }));
  process.exitCode = 1;
});
