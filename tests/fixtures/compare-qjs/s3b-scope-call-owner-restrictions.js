(() => {
  let results = []; class C {
    value = (() => {
      try { eval('arguments'); } catch(e) { results.push(e instanceof SyntaxError); }
      return function () { return eval('typeof arguments'); };
    })();
    static {
      const read = () => { try { eval('arguments'); } catch(e) { return e instanceof SyntaxError; } };
      results.push(read());
      results.push((function () { return eval('typeof arguments'); })());
      results.push(eval('new.target') === undefined);
    }
  }
  results.push(new C().value()); return results.join(':');
})()
