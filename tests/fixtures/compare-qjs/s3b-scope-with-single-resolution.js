(() => {
  const log = []; let missing = false;
  const p = new Proxy({}, {
    has(target, key) { log.push(String(key)); return false; },
    get() { throw new Error('unexpected Get'); }
  });
  with (p) { Object; typeof s3bUnresolvableName; }
  try { with (p) { s3bMissingReference; } } catch(e) { missing = e instanceof ReferenceError; }
  return missing + ':' + log.join('|');
})()
