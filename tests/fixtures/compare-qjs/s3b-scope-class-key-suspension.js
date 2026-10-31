(() => {
  function* g() {
    try {
      class C { #p; [(#p in {}, yield 'key')]() {} }
      return typeof new C().method;
    } finally { finalized++; }
  }
  let finalized = 0; const it = g();
  const first = it.next(); const last = it.next('method');
  const interrupted = g(); interrupted.next(); interrupted.return(8);
  return [first.value, last.value, last.done, finalized].join(':');
})()
